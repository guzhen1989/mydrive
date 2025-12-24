use tauri::State;
use serde::{Deserialize};
use crate::{AppState, error::Result, models::TransferTask};

#[tauri::command]
pub async fn get_transfer_tasks(state: State<'_, AppState>) -> Result<Vec<TransferTask>> {
    let db = state.db.lock().await;
    db.get_all_tasks()
}

#[tauri::command]
pub async fn pause_task(state: State<'_, AppState>, task_id: String) -> Result<()> {
    println!("Pausing task: {}", task_id);
    let db = state.db.lock().await;
    let mut task = db.get_task(&task_id)?
        .ok_or_else(|| crate::error::AppError::TaskNotFound(task_id.clone()))?;
    
    println!("Current task status: {:?}", task.status);
    if task.status == crate::models::TaskStatus::Running {
        task.status = crate::models::TaskStatus::Paused;
        db.update_task_status(&task_id, crate::models::TaskStatus::Paused)?;
        println!("Task paused successfully");
    }
    
    Ok(())
}

#[tauri::command]
pub async fn resume_task(state: State<'_, AppState>, task_id: String) -> Result<()> {
    println!("Resuming task: {}", task_id);
    let db = state.db.lock().await;
    let task = db.get_task(&task_id)?
        .ok_or_else(|| crate::error::AppError::TaskNotFound(task_id.clone()))?;
    
    println!("Current task status: {:?}", task.status);
    if task.status == crate::models::TaskStatus::Paused {
        // For now, we'll update the status in the database
        // In a full implementation, we would restart the actual transfer
        db.update_task_status(&task_id, crate::models::TaskStatus::Running)?;
        println!("Task resumed successfully");
    }
    
    Ok(())
}

#[tauri::command]
pub async fn cancel_task(state: State<'_, AppState>, task_id: String) -> Result<()> {
    println!("Cancelling task: {}", task_id);
    let db = state.db.lock().await;
    let result = db.update_task_status(&task_id, crate::models::TaskStatus::Cancelled)?;
    println!("Task cancelled successfully");
    Ok(result)
}

#[tauri::command]
pub async fn cancel_all_tasks(state: State<'_, AppState>) -> Result<()> {
    println!("Cancelling all tasks");
    let db = state.db.lock().await;
    let tasks = db.get_all_tasks()?;
    
    let mut cancelled_count = 0;
    for task in tasks {
        // Only cancel tasks that are running, paused, or pending
        if matches!(task.status, crate::models::TaskStatus::Running | crate::models::TaskStatus::Paused | crate::models::TaskStatus::Pending) {
            db.update_task_status(&task.task_id, crate::models::TaskStatus::Cancelled)?;
            cancelled_count += 1;
        }
    }
    
    println!("Cancelled {} tasks", cancelled_count);
    Ok(())
}

#[tauri::command]
pub async fn delete_task(state: State<'_, AppState>, task_id: String) -> Result<()> {
    println!("Deleting task: {}", task_id);
    let db = state.db.lock().await;
    db.delete_task(&task_id)?;
    println!("Task deleted successfully");
    Ok(())
}

#[tauri::command]
pub async fn delete_completed_tasks(state: State<'_, AppState>) -> Result<()> {
    println!("Deleting completed and cancelled tasks");
    let db = state.db.lock().await;
    let tasks = db.get_all_tasks()?;
    
    let mut deleted_count = 0;
    for task in tasks {
        // Delete completed, failed, and cancelled tasks
        if matches!(task.status, crate::models::TaskStatus::Completed | crate::models::TaskStatus::Failed | crate::models::TaskStatus::Cancelled) {
            db.delete_task(&task.task_id)?;
            deleted_count += 1;
        }
    }
    
    println!("Deleted {} tasks", deleted_count);
    Ok(())
}