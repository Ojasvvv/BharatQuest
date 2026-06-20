import re
with open("engine/src/runtime.rs", "r") as f:
    content = f.read()

# Add parse_output helper
content = content.replace("impl RuntimeHandle {", """fn parse_output(buf: &[u8]) -> (String, String) {
    if buf.is_empty() {
        return (String::new(), String::new());
    }
    let s = String::from_utf8_lossy(buf);
    if s.starts_with('e') {
        (String::new(), s[1..].to_string())
    } else if s.starts_with('s') {
        (s[1..].to_string(), String::new())
    } else {
        (s.to_string(), String::new())
    }
}

impl RuntimeHandle {""")

# Add runtime_notes to ExecutionResult in crate::ExecutionResult wait, ExecutionResult is defined where?
# In engine/src/lib.rs ?
