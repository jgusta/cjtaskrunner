fn format_taskfile_source(source: &str) -> String {
    if source.is_empty() {
        return String::new();
    }

    let mut formatted = String::new();
    for raw_line in source.lines() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let line = line.trim_end_matches([' ', '\t']);
        if line.is_empty() {
            formatted.push('\n');
            continue;
        }

        let (indent, content) = split_indent(line);
        if indent == 0 {
            formatted.push_str(content);
        } else {
            let normalized = normalize_indent(indent);
            formatted.push_str(&" ".repeat(normalized));
            formatted.push_str(content);
        }
        formatted.push('\n');
    }
    formatted
}

fn split_indent(line: &str) -> (usize, &str) {
    let mut width = 0;
    let mut end = 0;
    for (index, ch) in line.char_indices() {
        match ch {
            ' ' => {
                width += 1;
                end = index + 1;
            }
            '\t' => {
                width += 2;
                end = index + 1;
            }
            _ => break,
        }
    }
    (width, &line[end..])
}

fn normalize_indent(width: usize) -> usize {
    if width <= 2 {
        2
    } else {
        width - (width % 2)
    }
}
