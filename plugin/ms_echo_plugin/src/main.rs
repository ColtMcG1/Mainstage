use std::io::{self, Read};

fn main() {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_default();
    let func = args.next().unwrap_or_default();

    if cmd != "call" {
        eprintln!("unsupported command");
        std::process::exit(1);
    }

    // Read JSON request from stdin (we'll accept either full object or just args array)
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).unwrap_or(0);
    let json: serde_json::Value = if buf.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&buf).unwrap_or(serde_json::json!({}))
    };

    // For `echo`, return first arg as-is (or empty string)
    if func == "echo" {
        let out = match &json["args"] {
            serde_json::Value::Array(a) => a.get(0).cloned().unwrap_or(serde_json::Value::String("Unable to read input".to_string())),
            _ => serde_json::Value::String("Unknown input".to_string()),
        };
        // Print JSON-encoded output to stdout and keep logs on stderr.
        let s = serde_json::to_string(&out).unwrap_or("null".to_string());
        println!("{}", s);
        return;
    }

    eprintln!("unknown function");
    std::process::exit(1);
}
