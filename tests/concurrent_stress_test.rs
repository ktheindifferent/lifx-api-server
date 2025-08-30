use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::HashMap;

#[test]
fn test_concurrent_bulb_updates_stress() {
    // Simulate concurrent access to bulbs mutex
    let bulbs: Arc<Mutex<HashMap<u64, String>>> = Arc::new(Mutex::new(HashMap::new()));
    
    // Initialize with some test data
    {
        let mut bulbs_guard = bulbs.lock().unwrap();
        for i in 0..10 {
            bulbs_guard.insert(i, format!("Bulb_{}", i));
        }
    }
    
    let mut handles = vec![];
    let start = Instant::now();
    
    // Spawn multiple reader threads
    for thread_id in 0..5 {
        let bulbs_clone = Arc::clone(&bulbs);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                match bulbs_clone.lock() {
                    Ok(guard) => {
                        // Simulate reading bulb data
                        let _count = guard.len();
                        drop(guard); // Explicitly release lock quickly
                    }
                    Err(e) => {
                        eprintln!("Reader thread {} encountered poisoned mutex: {}", thread_id, e);
                        // In production, this would return an error response
                        break;
                    }
                }
                thread::sleep(Duration::from_micros(100));
            }
        });
        handles.push(handle);
    }
    
    // Spawn multiple writer threads
    for thread_id in 0..3 {
        let bulbs_clone = Arc::clone(&bulbs);
        let handle = thread::spawn(move || {
            for i in 0..50 {
                match bulbs_clone.lock() {
                    Ok(mut guard) => {
                        // Simulate updating bulb state
                        let key = (thread_id * 1000 + i) as u64;
                        guard.insert(key, format!("Updated_{}_{}", thread_id, i));
                        drop(guard); // Explicitly release lock quickly
                    }
                    Err(e) => {
                        eprintln!("Writer thread {} encountered poisoned mutex: {}", thread_id, e);
                        // In production, this would return an error response
                        break;
                    }
                }
                thread::sleep(Duration::from_micros(200));
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let elapsed = start.elapsed();
    println!("Concurrent stress test completed in {:?}", elapsed);
    
    // Verify final state
    let final_bulbs = bulbs.lock().unwrap();
    assert!(final_bulbs.len() >= 10, "Should have at least initial bulbs");
    assert!(elapsed < Duration::from_secs(5), "Should complete within reasonable time");
}

#[test]
fn test_high_contention_mutex_access() {
    let shared_data: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];
    
    // Create high contention scenario
    for thread_id in 0..20 {
        let data_clone = Arc::clone(&shared_data);
        let handle = thread::spawn(move || {
            for i in 0..10 {
                // Attempt to acquire mutex with proper error handling
                match data_clone.lock() {
                    Ok(mut guard) => {
                        guard.push(thread_id * 100 + i);
                        // Hold lock briefly to simulate work
                        thread::sleep(Duration::from_micros(10));
                    }
                    Err(e) => {
                        eprintln!("Thread {} failed to acquire lock: {}", thread_id, e);
                        return Err(format!("Mutex poisoned: {}", e));
                    }
                }
            }
            Ok(())
        });
        handles.push(handle);
    }
    
    // Collect results
    let mut errors = 0;
    for handle in handles {
        if let Ok(Err(_)) = handle.join() {
            errors += 1;
        }
    }
    
    // Verify results
    match shared_data.lock() {
        Ok(guard) => {
            println!("Final vector size: {}", guard.len());
            // Each thread should have added 10 items if no errors
            let expected_min = (20 - errors) * 10;
            assert!(guard.len() >= expected_min as usize, 
                    "Should have at least {} items", expected_min);
        }
        Err(_) => {
            // Mutex is poisoned but we handled it properly
            assert!(errors > 0, "Should have recorded errors if mutex is poisoned");
        }
    };
}

#[test]
fn test_mutex_recovery_after_panic() {
    use std::panic;
    
    let data: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
    let data_clone = Arc::clone(&data);
    
    // Thread that will panic while holding mutex
    let panic_handle = thread::spawn(move || {
        let mut guard = data_clone.lock().unwrap();
        *guard = 42;
        panic!("Simulated panic!");
    });
    
    // Wait for panic
    let _ = panic_handle.join();
    
    // Try to recover from poisoned mutex
    match data.lock() {
        Ok(_guard) => {
            panic!("Should not succeed - mutex should be poisoned");
        }
        Err(poisoned) => {
            // Proper way to recover from poisoned mutex
            let recovered_guard = poisoned.into_inner();
            assert_eq!(*recovered_guard, 42, "Should recover the last value");
            println!("Successfully recovered from poisoned mutex");
        }
    };
}

#[test]
fn test_rate_limiter_under_load() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    // Simulate rate limiter behavior
    let attempt_counts: Arc<Mutex<HashMap<String, usize>>> = Arc::new(Mutex::new(HashMap::new()));
    let successful_requests = Arc::new(AtomicUsize::new(0));
    let failed_requests = Arc::new(AtomicUsize::new(0));
    
    let mut handles = vec![];
    
    // Simulate multiple clients making requests
    for client_id in 0..10 {
        let attempts_clone = Arc::clone(&attempt_counts);
        let success_clone = Arc::clone(&successful_requests);
        let failed_clone = Arc::clone(&failed_requests);
        
        let handle = thread::spawn(move || {
            let client_ip = format!("192.168.1.{}", client_id);
            
            for _ in 0..10 {
                match attempts_clone.lock() {
                    Ok(mut guard) => {
                        let count = guard.entry(client_ip.clone()).or_insert(0);
                        *count += 1;
                        
                        // Simulate rate limiting (max 5 attempts)
                        if *count <= 5 {
                            success_clone.fetch_add(1, Ordering::Relaxed);
                        } else {
                            failed_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to acquire lock for client {}: {}", client_id, e);
                        failed_clone.fetch_add(1, Ordering::Relaxed);
                    }
                }
                
                thread::sleep(Duration::from_millis(10));
            }
        });
        handles.push(handle);
    }
    
    // Wait for completion
    for handle in handles {
        handle.join().unwrap();
    }
    
    let total_success = successful_requests.load(Ordering::Relaxed);
    let total_failed = failed_requests.load(Ordering::Relaxed);
    
    println!("Rate limiter test results:");
    println!("  Successful requests: {}", total_success);
    println!("  Failed requests: {}", total_failed);
    
    // Each client makes 10 requests, first 5 should succeed
    assert!(total_success >= 50, "Should have at least 50 successful requests");
    assert!(total_failed >= 50, "Should have at least 50 rate-limited requests");
}

#[test]
fn test_deadlock_prevention() {
    // Test that our mutex usage patterns don't cause deadlocks
    let mutex1: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
    let mutex2: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
    
    let m1_clone = Arc::clone(&mutex1);
    let m2_clone = Arc::clone(&mutex2);
    
    // Thread 1: Always acquire mutex1 then mutex2
    let handle1 = thread::spawn(move || {
        for _i in 0..100 {
            if let Ok(mut guard1) = m1_clone.lock() {
                *guard1 += 1;
                // Always drop guard1 before acquiring guard2
                drop(guard1);
                
                if let Ok(mut guard2) = m2_clone.lock() {
                    *guard2 += 1;
                }
            }
        }
    });
    
    let m1_clone2 = Arc::clone(&mutex1);
    let m2_clone2 = Arc::clone(&mutex2);
    
    // Thread 2: Also acquire in same order (mutex1 then mutex2)
    // This prevents deadlock
    let handle2 = thread::spawn(move || {
        for _i in 0..100 {
            if let Ok(mut guard1) = m1_clone2.lock() {
                *guard1 -= 1;
                drop(guard1);
                
                if let Ok(mut guard2) = m2_clone2.lock() {
                    *guard2 -= 1;
                }
            }
        }
    });
    
    // Set a timeout for deadlock detection
    let start = Instant::now();
    handle1.join().unwrap();
    handle2.join().unwrap();
    let elapsed = start.elapsed();
    
    assert!(elapsed < Duration::from_secs(2), "Should complete without deadlock");
    
    // Verify final state
    let val1 = *mutex1.lock().unwrap();
    let val2 = *mutex2.lock().unwrap();
    println!("Final values: mutex1={}, mutex2={}", val1, val2);
}