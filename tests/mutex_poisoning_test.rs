use std::sync::{Arc, Mutex};
use std::thread;
use std::panic;
use std::time::Duration;

extern crate lifx_api_server;
use lifx_api_server::mutex_utils::{
    safe_lock, safe_lock_monitored, safe_lock_with_recovery, 
    safe_try_lock, MUTEX_MONITOR
};

#[test]
fn test_safe_lock_recovers_from_poisoning() {
    let data: Arc<Mutex<i32>> = Arc::new(Mutex::new(100));
    let data_clone = Arc::clone(&data);
    
    // Thread that panics while holding the lock
    let handle = thread::spawn(move || {
        let mut guard = data_clone.lock().unwrap();
        *guard = 200;
        panic!("Intentional panic to poison mutex");
    });
    
    // Wait for the thread to panic
    let _ = handle.join();
    
    // Use safe_lock to recover from poisoned mutex
    let result = safe_lock(&data);
    assert!(result.is_ok(), "safe_lock should recover from poisoned mutex");
    
    let guard = result.unwrap();
    assert_eq!(*guard, 200, "Should recover the value that was set before panic");
}

#[test]
fn test_safe_lock_with_custom_recovery() {
    #[derive(Debug, Clone)]
    struct State {
        value: i32,
        valid: bool,
    }
    
    let data: Arc<Mutex<State>> = Arc::new(Mutex::new(State { value: 0, valid: true }));
    let data_clone = Arc::clone(&data);
    
    // Thread that panics after partially modifying state
    let handle = thread::spawn(move || {
        let mut guard = data_clone.lock().unwrap();
        guard.value = 999;
        guard.valid = false; // Invalid state
        panic!("Panic with invalid state");
    });
    
    let _ = handle.join();
    
    // Use safe_lock_with_recovery to fix the invalid state
    let result = safe_lock_with_recovery(&data, |state| {
        if !state.valid {
            // Reset to a known good state
            state.value = 0;
            state.valid = true;
        }
    });
    
    assert!(result.is_ok(), "Should recover with custom logic");
    let guard = result.unwrap();
    assert_eq!(guard.value, 0, "Should reset to safe value");
    assert!(guard.valid, "Should be in valid state");
}

#[test]
fn test_safe_try_lock() {
    let data: Arc<Mutex<i32>> = Arc::new(Mutex::new(42));
    
    // Normal try_lock
    let result = safe_try_lock(&data);
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
    
    // Try lock when already locked
    let _guard = data.lock().unwrap();
    let data_clone = Arc::clone(&data);
    
    let handle = thread::spawn(move || {
        let result = safe_try_lock(&data_clone);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none(), "Should return None when locked");
    });
    
    handle.join().unwrap();
}

#[test]
fn test_monitoring_tracks_poisoning_events() {
    let initial_count = MUTEX_MONITOR.get_poisoning_count();
    
    let data: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(vec![1, 2, 3]));
    
    // Cause multiple poisoning events
    for i in 0..3 {
        let data_clone = Arc::clone(&data);
        let handle = thread::spawn(move || {
            let mut guard = data_clone.lock().unwrap();
            guard.push(i);
            panic!("Panic #{}", i);
        });
        let _ = handle.join();
        
        // Recover using monitored lock
        let _guard = safe_lock_monitored(&data, &format!("test_mutex_{}", i));
    }
    
    // Check that monitoring tracked the events
    let final_count = MUTEX_MONITOR.get_poisoning_count();
    assert_eq!(final_count - initial_count, 3, "Should track 3 poisoning events");
    assert!(MUTEX_MONITOR.get_last_poisoning().is_some(), "Should record last poisoning time");
}

#[test]
fn test_concurrent_recovery_stress() {
    let shared_data: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];
    
    // Mix of normal threads and panicking threads
    for thread_id in 0..10 {
        let data_clone = Arc::clone(&shared_data);
        
        let handle = thread::spawn(move || {
            for i in 0..5 {
                // Every 3rd iteration of every 3rd thread will panic
                let should_panic = thread_id % 3 == 0 && i % 3 == 0 && i > 0;
                
                if should_panic {
                    // This thread will panic and poison the mutex
                    let mut guard = data_clone.lock().unwrap();
                    guard.push(thread_id * 100 + i);
                    panic!("Intentional panic from thread {}", thread_id);
                } else {
                    // Use safe_lock to handle potential poisoning
                    match safe_lock(&data_clone) {
                        Ok(mut guard) => {
                            guard.push(thread_id * 100 + i);
                        }
                        Err(e) => {
                            eprintln!("Thread {} failed to lock: {}", thread_id, e);
                        }
                    }
                }
                
                thread::sleep(Duration::from_micros(100));
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        let _ = handle.join(); // Some will panic, that's expected
    }
    
    // Final verification using safe_lock
    let result = safe_lock(&shared_data);
    assert!(result.is_ok(), "Should be able to recover final state");
    
    let guard = result.unwrap();
    println!("Final vector length after stress test: {}", guard.len());
    assert!(!guard.is_empty(), "Should have some data despite panics");
}

#[test]
fn test_nested_mutex_recovery() {
    #[derive(Debug)]
    struct Container {
        inner: Mutex<i32>,
    }
    
    let container: Arc<Mutex<Container>> = Arc::new(Mutex::new(Container {
        inner: Mutex::new(0),
    }));
    
    let container_clone = Arc::clone(&container);
    
    // Poison the outer mutex
    let handle = thread::spawn(move || {
        let outer_guard = container_clone.lock().unwrap();
        let mut inner_guard = outer_guard.inner.lock().unwrap();
        *inner_guard = 42;
        drop(inner_guard);
        panic!("Poisoning outer mutex");
    });
    
    let _ = handle.join();
    
    // Recover using safe_lock
    let outer_result = safe_lock(&container);
    assert!(outer_result.is_ok(), "Should recover outer mutex");
    
    let outer_guard = outer_result.unwrap();
    let inner_result = safe_lock(&outer_guard.inner);
    assert!(inner_result.is_ok(), "Should handle inner mutex");
    
    let inner_guard = inner_result.unwrap();
    assert_eq!(*inner_guard, 42, "Should preserve inner value");
}

#[test]
fn test_recovery_preserves_data_integrity() {
    use std::collections::HashMap;
    
    let map: Arc<Mutex<HashMap<String, i32>>> = Arc::new(Mutex::new(HashMap::new()));
    
    // Initialize with some data
    {
        let mut guard = safe_lock(&map).unwrap();
        guard.insert("key1".to_string(), 100);
        guard.insert("key2".to_string(), 200);
    }
    
    let map_clone = Arc::clone(&map);
    
    // Panic during modification
    let handle = thread::spawn(move || {
        let mut guard = map_clone.lock().unwrap();
        guard.insert("key3".to_string(), 300);
        // Panic before completing all operations
        panic!("Panic during map modification");
    });
    
    let _ = handle.join();
    
    // Recover and verify data integrity
    let result = safe_lock_with_recovery(&map, |data| {
        // Verify and potentially fix data
        if data.contains_key("key3") && !data.contains_key("key4") {
            // Incomplete transaction detected, could rollback or complete
            println!("Detected incomplete transaction, data is: {:?}", data);
        }
    });
    
    assert!(result.is_ok());
    let guard = result.unwrap();
    assert_eq!(guard.get("key1"), Some(&100));
    assert_eq!(guard.get("key2"), Some(&200));
    assert_eq!(guard.get("key3"), Some(&300)); // Partial update was preserved
}

#[test]
fn test_safe_lock_performance_overhead() {
    use std::time::Instant;
    
    let data: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
    let iterations = 10000;
    
    // Measure normal lock performance
    let start = Instant::now();
    for _ in 0..iterations {
        if let Ok(mut guard) = data.lock() {
            *guard += 1;
        }
    }
    let normal_duration = start.elapsed();
    
    // Reset
    *data.lock().unwrap() = 0;
    
    // Measure safe_lock performance
    let start = Instant::now();
    for _ in 0..iterations {
        if let Ok(mut guard) = safe_lock(&data) {
            *guard += 1;
        }
    }
    let safe_duration = start.elapsed();
    
    println!("Normal lock: {:?}, Safe lock: {:?}", normal_duration, safe_duration);
    
    // Safe lock should not be significantly slower (allow 2x overhead)
    assert!(safe_duration < normal_duration * 2, 
            "Safe lock overhead should be reasonable");
    
    // Verify correctness
    let final_value = *safe_lock(&data).unwrap();
    assert_eq!(final_value, iterations as i32);
}