//! file: core/src/vm/host.rs
//! description: built-in host function implementations used by the VM.
//!
//! Simple host-call implementations (e.g. `fmt`, `say`, `read`, `write`)
//! are provided here. These are invoked by the runtime when bytecode issues
//! host-level calls represented as `Value::Symbol` names.
//!
use crate::vm::value::Value;
use glob::glob;
use std::fs;

pub(crate) fn run_host_fn(name: &str, args: &Vec<Value>) -> Result<Value, String> {
    match name {
        "fmt" => {
            // Implementation moved here from run.rs; keep same semantics
            if let Some(Value::Str(fmt_str)) = args.get(0) {
                let mut out = String::new();
                let mut arg_idx: usize = 1;
                let chars: Vec<char> = fmt_str.chars().collect();
                let mut i: usize = 0;
                while i < chars.len() {
                    let ch = chars[i];
                    if ch == '{' {
                        // escaped '{{'
                        if i + 1 < chars.len() && chars[i + 1] == '{' {
                            out.push('{');
                            i += 2;
                            continue;
                        }

                        // collect contiguous specs into a vector
                        // each spec: (fill_char, align_char, width, precision)
                        let mut specs: Vec<(Option<char>, Option<char>, Option<usize>, Option<usize>)> = Vec::new();
                        loop {
                            if i >= chars.len() || chars[i] != '{' {
                                break;
                            }
                            // find matching '}'
                            let mut j = i + 1;
                            let mut spec_body = String::new();
                            while j < chars.len() && chars[j] != '}' {
                                spec_body.push(chars[j]);
                                j += 1;
                            }
                            if j >= chars.len() {
                                return Err("fmt: unmatched '{' in format string".to_string());
                            }

                            // parse spec_body into fill/align/width/precision
                            let mut fill: Option<char> = None;
                            let mut align: Option<char> = None;
                            let mut width: Option<usize> = None;
                            let mut precision: Option<usize> = None;
                            if spec_body.starts_with(':') {
                                let mut body = spec_body[1..].to_string();
                                // detect [fill][align]
                                if body.len() >= 2 {
                                    let mut cs = body.chars();
                                    let c0 = cs.next().unwrap();
                                    let c1 = cs.next().unwrap();
                                    if matches!(c1, '<' | '>' | '^') {
                                        fill = Some(c0);
                                        align = Some(c1);
                                        body = cs.collect();
                                    }
                                }
                                // detect align alone
                                if align.is_none() && body.len() >= 1 {
                                    let mut cs = body.chars();
                                    let c0 = cs.next().unwrap();
                                    if matches!(c0, '<' | '>' | '^') {
                                        align = Some(c0);
                                        body = cs.collect();
                                    }
                                }

                                if body.len() > 0 {
                                    if let Some(dot_pos) = body.find('.') {
                                        let (wpart, ppart) = body.split_at(dot_pos);
                                        if wpart.len() > 0 {
                                            if wpart.starts_with('0') {
                                                fill = Some('0');
                                            }
                                            if let Ok(w) = wpart.parse::<usize>() {
                                                width = Some(w);
                                            }
                                        }
                                        let pstr = &ppart[1..];
                                        if pstr.len() > 0 {
                                            if let Ok(p) = pstr.parse::<usize>() {
                                                precision = Some(p);
                                            }
                                        }
                                    } else {
                                        let wpart = body;
                                        if wpart.len() > 0 {
                                            if wpart.starts_with('0') {
                                                fill = Some('0');
                                            }
                                            if let Ok(w) = wpart.parse::<usize>() {
                                                width = Some(w);
                                            }
                                        }
                                    }
                                }
                            }

                            specs.push((fill, align, width, precision));
                            i = j + 1; // advance past '}'

                            // if next char is '{' and not an escaped '{{', loop to parse another spec
                            if i < chars.len() && chars[i] == '{' {
                                if i + 1 < chars.len() && chars[i + 1] == '{' {
                                    break;
                                } else {
                                    continue;
                                }
                            }
                            break;
                        }

                        // decide whether to apply specs separately (consuming args for each)
                        let remaining_args = if args.len() > arg_idx { args.len() - arg_idx } else { 0 };
                        if remaining_args >= specs.len() {
                            // format each spec with its own arg
                            for (fch, aching, pw, pp) in specs.iter() {
                                let val = args.get(arg_idx).unwrap_or(&Value::Null).clone();
                                arg_idx += 1;
                                let mut s = match val {
                                    Value::Int(i) => i.to_string(),
                                    Value::Float(f) => {
                                        if let Some(p) = pp { format!("{:.1$}", f, *p) } else { format!("{}", f) }
                                    }
                                    Value::Str(st) => st.clone(),
                                    Value::Symbol(st) => st.clone(),
                                    Value::Bool(b) => b.to_string(),
                                    Value::Null => "null".to_string(),
                                    other => format!("{:?}", other.to_value()),
                                };
                                if let Some(wv) = pw {
                                    let wv = *wv;
                                    let len = s.chars().count();
                                    if len < wv {
                                        let pad_char = fch.unwrap_or(' ');
                                        let pad_len = wv - len;
                                        match aching.unwrap_or('>') {
                                            '<' => {
                                                let mut pad = String::new();
                                                for _ in 0..pad_len { pad.push(pad_char); }
                                                s = format!("{}{}", s, pad);
                                            }
                                            '^' => {
                                                let left = pad_len / 2;
                                                let right = pad_len - left;
                                                let mut lpad = String::new();
                                                let mut rpad = String::new();
                                                for _ in 0..left { lpad.push(pad_char); }
                                                for _ in 0..right { rpad.push(pad_char); }
                                                s = format!("{}{}{}", lpad, s, rpad);
                                            }
                                            _ => {
                                                let mut pad = String::new();
                                                for _ in 0..pad_len { pad.push(pad_char); }
                                                s = format!("{}{}", pad, s);
                                            }
                                        }
                                    }
                                }
                                out.push_str(&s);
                            }
                        } else {
                            // merge specs into a single spec and apply to one arg
                            let mut merged_fill: Option<char> = None;
                            let mut merged_align: Option<char> = None;
                            let mut merged_width: Option<usize> = None;
                            let mut merged_prec: Option<usize> = None;
                            for (fch, aching, pw, pp) in specs.iter() {
                                if merged_fill.is_none() { merged_fill = *fch; }
                                if merged_align.is_none() { merged_align = *aching; }
                                if merged_width.is_none() { merged_width = *pw; }
                                if merged_prec.is_none() { merged_prec = *pp; }
                            }
                            let val = if arg_idx < args.len() { let v = args.get(arg_idx).unwrap().clone(); arg_idx += 1; v } else if args.len() > 1 { args.get(args.len() - 1).unwrap().clone() } else { Value::Null };
                            let mut s = match val {
                                Value::Int(i) => i.to_string(),
                                Value::Float(f) => {
                                    if let Some(pp) = merged_prec { format!("{:.1$}", f, pp) } else { format!("{}", f) }
                                }
                                Value::Str(st) => st.clone(),
                                Value::Symbol(st) => st.clone(),
                                Value::Bool(b) => b.to_string(),
                                Value::Null => "null".to_string(),
                                other => format!("{:?}", other.to_value()),
                            };
                            if let Some(wv) = merged_width {
                                let len = s.chars().count();
                                if len < wv {
                                    let pad_char = merged_fill.unwrap_or(' ');
                                    let pad_len = wv - len;
                                    match merged_align.unwrap_or('>') {
                                        '<' => {
                                            let mut pad = String::new();
                                            for _ in 0..pad_len { pad.push(pad_char); }
                                            s = format!("{}{}", s, pad);
                                        }
                                        '^' => {
                                            let left = pad_len / 2;
                                            let right = pad_len - left;
                                            let mut lpad = String::new();
                                            let mut rpad = String::new();
                                            for _ in 0..left { lpad.push(pad_char); }
                                            for _ in 0..right { rpad.push(pad_char); }
                                            s = format!("{}{}{}", lpad, s, rpad);
                                        }
                                        _ => {
                                            let mut pad = String::new();
                                            for _ in 0..pad_len { pad.push(pad_char); }
                                            s = format!("{}{}", pad, s);
                                        }
                                    }
                                }
                            }
                            out.push_str(&s);
                        }
                    } else if ch == '}' {
                        // escaped '}}'
                        if i + 1 < chars.len() && chars[i + 1] == '}' {
                            out.push('}');
                            i += 2;
                            continue;
                        } else {
                            return Err("fmt: unmatched '}' in format string".to_string());
                        }
                    } else {
                        out.push(ch);
                        i += 1;
                    }
                }

                return Ok(Value::Str(out));
            } else {
                return Err("fmt: first argument must be a format string".to_string());
            }
        }
        "ask" => {
            use std::io::{self, Write};
            if let Some(Value::Str(prompt)) = args.get(0) {
                print!("{}", prompt);
                io::stdout()
                    .flush()
                    .map_err(|e| format!("io error: {}", e))?;
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| format!("io error: {}", e))?;
                let s = input.trim_end().to_string();
                let s_trim = s.trim();
                // Try boolean
                let low = s_trim.to_ascii_lowercase();
                if low == "true" {
                    return Ok(Value::Bool(true));
                } else if low == "false" {
                    return Ok(Value::Bool(false));
                }
                // Try integer
                if let Ok(i) = s_trim.parse::<i64>() {
                    return Ok(Value::Int(i));
                }
                // Try float
                if let Ok(f) = s_trim.parse::<f64>() {
                    return Ok(Value::Float(f));
                }
                // Fallback to string
                Ok(Value::Str(s))
            } else {
                Ok(Value::Str(String::new()))
            }
        }
        "say" => {
            if let Some(a) = args.get(0) {
                match a {
                    Value::Str(s) => println!("{}", s),
                    Value::Symbol(s) => println!("{}", s),
                    Value::Array(arr) => {
                        // Print each array element on its own line if possible
                        for item in arr {
                            match item {
                                Value::Str(s) => println!("{}", s),
                                Value::Symbol(sym) => println!("{}", sym),
                                other => println!("{:?}", other.to_value()),
                            }
                        }
                    }
                    _ => println!("{:?}", a.to_value()),
                }
            }
            Ok(Value::Null)
        }
        "read" => {
            if let Some(Value::Str(glob_pat)) = args.get(0) {
                match glob(glob_pat) {
                    Ok(paths) => {
                        let mut out: Vec<Value> = Vec::new();
                        for p in paths.flatten() {
                            if let Ok(s) = fs::read_to_string(&p) {
                                out.push(Value::Str(s));
                            }
                        }
                        // Return an array (possibly empty) of file contents
                        Ok(Value::Array(out))
                    }
                    Err(e) => Err(format!("glob error: {}", e)),
                }
            } else {
                Ok(Value::Array(Vec::new()))
            }
        }
        "write" => {
            if let (Some(Value::Str(path)), Some(Value::Str(content))) = (args.get(0), args.get(1))
            {
                match fs::write(path, content) {
                    Ok(_) => Ok(Value::Bool(true)),
                    Err(e) => Err(format!("write error: {}", e)),
                }
            } else {
                Err("write: invalid arguments".to_string())
            }
        }
        _ => Err(format!("unknown host function: {}", name)),
    }
}
