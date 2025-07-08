use sentient_shell::services::activity_loop::ActivityLoopService;
use sentient_shell::services::SentientService;
use tokio::time::{sleep, Duration};
use std::path::PathBuf;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use chrono::Utc;

#[tokio::test]
async fn test_activity_loop_integration() {
    // Create test logs directory
    let test_logs_dir = PathBuf::from("test_logs");
    create_dir_all(&test_logs_dir).unwrap();
    
    // Create test goal injection file
    let injection_file = test_logs_dir.join("goal_injections.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&injection_file)
        .unwrap();
    
    // Write test goals
    writeln!(file, r#"{{"goal":"Check memory usage","source":"test","timestamp":"{}","processed":false}}"#, 
        Utc::now().to_rfc3339()).unwrap();
    writeln!(file, r#"{{"goal":"Monitor CPU load","source":"test","timestamp":"{}","processed":false}}"#, 
        Utc::now().to_rfc3339()).unwrap();
    
    // Create and initialize service
    let mut service = ActivityLoopService::new();
    
    // Test service initialization
    assert!(service.init().await.is_ok());
    assert_eq!(service.name(), "activity-loop");
    
    // Run service for a short time in a spawned task
    let service_handle = tokio::spawn(async move {
        let _ = tokio::time::timeout(
            Duration::from_secs(2),
            service.run()
        ).await;
    });
    
    // Give service time to process goals
    sleep(Duration::from_secs(1)).await;
    
    // Check that log file was created
    let log_file = test_logs_dir.join(format!("activity_loop_log_{}.jsonl", 
        Utc::now().format("%Y%m%d")));
    assert!(log_file.exists(), "Log file should be created");
    
    // Clean up
    let _ = service_handle.await;
    let _ = std::fs::remove_dir_all(&test_logs_dir);
}

#[tokio::test]
async fn test_goal_processing() {
    use sentient_shell::services::activity_loop::{ActivityLoopService, ActivityGoalEntry};
    
    let service = ActivityLoopService::new();
    
    // Test goal entry
    let goal_entry = ActivityGoalEntry {
        goal: "Check system health and resource usage".to_string(),
        source: "test".to_string(),
        timestamp: Utc::now(),
        processed: false,
        priority: Some("high".to_string()),
    };
    
    // Process goal (using private method through testing)
    // Note: In real implementation, we'd expose this through a trait or testing module
    let command = "echo 'Health check output'";
    let (output, success, _) = service.execute_command(command).await;
    
    assert!(success);
    assert!(output.contains("Health check output"));
}

#[test]
fn test_command_mapping() {
    use sentient_shell::services::activity_loop::ActivityLoopService;
    
    let service = ActivityLoopService::new();
    
    // Test various goal mappings
    let test_cases = vec![
        ("Check disk activity", "df -h"),
        ("Monitor memory usage", "free -h"),
        ("Check network connections", "netstat"),
        ("Monitor CPU load", "uptime"),
        ("Count running processes", "ps aux"),
        ("Check system health", "uptime -p"),
        ("Analyze error logs", "find logs"),
        ("Check service status", "ps aux | grep"),
    ];
    
    for (goal, expected_cmd_part) in test_cases {
        let cmd = service.goal_to_command(goal);
        assert!(cmd.contains(expected_cmd_part), 
            "Goal '{}' should map to command containing '{}'", goal, expected_cmd_part);
    }
}

#[test]
fn test_reward_calculation() {
    use sentient_shell::services::activity_loop::ActivityLoopService;
    
    let service = ActivityLoopService::new();
    
    // Test various output scenarios
    assert_eq!(service.calculate_reward("", false), 0.0);
    assert_eq!(service.calculate_reward("short", true), 0.3);
    
    // Long output with structure
    let good_output = "Memory: Total 16G, Used 8G, Free 8G\nCPU Load: 2.5";
    let reward = service.calculate_reward(good_output, true);
    assert!(reward > 0.7, "Good output should have high reward");
    
    // Output with error
    let error_output = "Error: Command not found";
    let reward = service.calculate_reward(error_output, true);
    assert!(reward < 0.5, "Error output should have lower reward");
}