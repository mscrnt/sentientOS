//! Test prefix command validation

use sentient_shell::validated_exec::execute_with_prefix;

fn main() {
    println!("=== Testing Prefix Commands ===\n");
    
    let test_commands = vec![
        "!@ service list",
        "!# rm -rf /tmp/test",
        "!$ systemctl restart nginx",
        "!& backup.sh",
        "!~ python test.py",
        "normal command",
    ];
    
    for cmd in test_commands {
        println!("Testing: {}", cmd);
        match execute_with_prefix(cmd) {
            Ok(_) => println!("  ✓ Processed successfully\n"),
            Err(e) => println!("  ✗ Error: {}\n", e),
        }
    }
}