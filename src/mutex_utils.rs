use std::sync::{Mutex, MutexGuard};
use log::{error, warn};

/// Safe mutex lock with automatic recovery from poisoning.
/// 
/// This function attempts to lock a mutex and handles poisoning gracefully.
/// If the mutex is poisoned, it logs the event and recovers the guard.
pub fn safe_lock<'a, T>(mutex: &'a Mutex<T>) -> Result<MutexGuard<'a, T>, String> {
    match mutex.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            error!("Mutex poisoned, recovering... Thread that poisoned: {:?}", 
                   std::thread::current().id());
            
            // Recover the guard - this is safe because we're acknowledging
            // that the data might be in an inconsistent state
            Ok(poisoned.into_inner())
        }
    }
}

/// Safe mutex lock with custom recovery logic.
/// 
/// This function allows you to provide a recovery function that will be called
/// if the mutex is poisoned. The recovery function can inspect and potentially
/// fix the data before returning.
pub fn safe_lock_with_recovery<'a, T, F>(
    mutex: &'a Mutex<T>,
    recovery: F,
) -> Result<MutexGuard<'a, T>, String>
where
    F: FnOnce(&mut T),
{
    match mutex.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            warn!("Mutex poisoned, applying recovery logic...");
            
            let mut guard = poisoned.into_inner();
            recovery(&mut *guard);
            
            Ok(guard)
        }
    }
}

/// Try to lock a mutex without blocking, with poisoning recovery.
pub fn safe_try_lock<'a, T>(mutex: &'a Mutex<T>) -> Result<Option<MutexGuard<'a, T>>, String> {
    match mutex.try_lock() {
        Ok(guard) => Ok(Some(guard)),
        Err(e) => {
            if let std::sync::TryLockError::Poisoned(poisoned) = e {
                error!("Mutex poisoned during try_lock, recovering...");
                Ok(Some(poisoned.into_inner()))
            } else {
                // WouldBlock - mutex is currently locked by another thread
                Ok(None)
            }
        }
    }
}

/// Monitoring structure for tracking mutex poisoning events.
#[derive(Debug, Default)]
pub struct MutexMonitor {
    poisoning_count: std::sync::atomic::AtomicUsize,
    last_poisoning: Mutex<Option<std::time::Instant>>,
}

impl MutexMonitor {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn record_poisoning(&self) {
        self.poisoning_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut last) = self.last_poisoning.lock() {
            *last = Some(std::time::Instant::now());
        }
    }
    
    pub fn get_poisoning_count(&self) -> usize {
        self.poisoning_count.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    pub fn get_last_poisoning(&self) -> Option<std::time::Instant> {
        self.last_poisoning.lock().ok().and_then(|guard| *guard)
    }
}

// Global monitor for tracking mutex poisoning events
lazy_static::lazy_static! {
    pub static ref MUTEX_MONITOR: MutexMonitor = MutexMonitor::new();
}

/// Enhanced safe lock with monitoring
pub fn safe_lock_monitored<'a, T>(mutex: &'a Mutex<T>, name: &str) -> Result<MutexGuard<'a, T>, String> {
    match mutex.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            error!("Mutex '{}' poisoned, recovering...", name);
            MUTEX_MONITOR.record_poisoning();
            
            Ok(poisoned.into_inner())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    
    #[test]
    fn test_safe_lock_normal() {
        let mutex = Mutex::new(42);
        let guard = safe_lock(&mutex).unwrap();
        assert_eq!(*guard, 42);
    }
    
    #[test]
    fn test_safe_lock_poisoned() {
        let mutex = Arc::new(Mutex::new(42));
        let mutex_clone = mutex.clone();
        
        // Poison the mutex by panicking while holding the lock
        let handle = thread::spawn(move || {
            let _guard = mutex_clone.lock().unwrap();
            panic!("Intentional panic to poison mutex");
        });
        
        // Wait for the thread to panic
        let _ = handle.join();
        
        // Now try to lock the poisoned mutex
        let guard = safe_lock(&mutex).unwrap();
        assert_eq!(*guard, 42); // Should still be able to access the data
    }
    
    #[test]
    fn test_safe_lock_with_recovery() {
        let mutex = Arc::new(Mutex::new(vec![1, 2, 3]));
        let mutex_clone = mutex.clone();
        
        // Poison the mutex
        let handle = thread::spawn(move || {
            let mut guard = mutex_clone.lock().unwrap();
            guard.push(4);
            panic!("Intentional panic");
        });
        
        let _ = handle.join();
        
        // Lock with recovery - reset to safe state
        let guard = safe_lock_with_recovery(&mutex, |data| {
            // Recovery logic: ensure vector has at most 3 elements
            data.truncate(3);
        }).unwrap();
        
        assert_eq!(*guard, vec![1, 2, 3]);
    }
    
    #[test]
    fn test_monitoring() {
        let initial_count = MUTEX_MONITOR.get_poisoning_count();
        
        let mutex = Arc::new(Mutex::new(0));
        let mutex_clone = mutex.clone();
        
        // Poison the mutex
        let handle = thread::spawn(move || {
            let _guard = mutex_clone.lock().unwrap();
            panic!("Test panic");
        });
        
        let _ = handle.join();
        
        // Use monitored lock
        let _guard = safe_lock_monitored(&mutex, "test_mutex").unwrap();
        
        assert_eq!(MUTEX_MONITOR.get_poisoning_count(), initial_count + 1);
        assert!(MUTEX_MONITOR.get_last_poisoning().is_some());
    }
}