use std::path::Path;

/// Get icon for a file based on its name and extension  
pub fn get_file_icon(filename: &str) -> char {
    // Check special filenames first
    match filename {
        // Rust
        "Cargo.toml" | "Cargo.lock" => '\u{e7a8}', //

        // Git
        ".gitignore" | ".gitmodules" | ".gitattributes" => '\u{f1d3}', //

        // Build files
        "Makefile" | "makefile" => '\u{e779}', //
        "CMakeLists.txt" => '\u{e779}',        //

        // Config
        ".editorconfig" => '\u{e615}', //

        // Documentation
        "README" | "README.md" => '\u{f48a}',  //
        "LICENSE" | "CHANGELOG" => '\u{f15c}', //
        "CHANGELOG.md" => '\u{f48a}',          //

        _ => {
            // Check extension
            if let Some(extension) = Path::new(filename).extension() {
                if let Some(ext_str) = extension.to_str() {
                    match ext_str.to_lowercase().as_str() {
                        // Programming languages
                        "rs" => '\u{e7a8}',                                 //
                        "py" | "pyc" | "pyo" | "pyw" => '\u{e73c}',         //
                        "js" | "jsx" | "mjs" => '\u{e74e}',                 //
                        "ts" | "tsx" => '\u{e628}',                         //
                        "go" => '\u{e724}',                                 //
                        "java" | "class" | "jar" => '\u{e738}',             //
                        "c" | "h" => '\u{e61e}',                            //
                        "cpp" | "cxx" | "cc" | "hpp" | "hxx" => '\u{e61d}', //
                        "rb" => '\u{e739}',                                 //

                        // Config
                        "json" => '\u{e60b}',                 //
                        "yaml" | "yml" => '\u{f481}',         //
                        "toml" => '\u{e615}',                 //
                        "ini" | "conf" | "cfg" => '\u{e615}', //

                        // Documentation
                        "md" | "markdown" => '\u{f48a}', //
                        "txt" | "text" => '\u{f15c}',    //

                        // Web
                        "html" | "htm" => '\u{e736}',          //
                        "css" | "scss" | "sass" => '\u{e749}', //

                        // Default
                        _ => '\u{f15b}', //
                    }
                } else {
                    '\u{f15b}' // 
                }
            } else {
                '\u{f15b}' // No extension 
            }
        }
    }
}

/// Get icon for a directory
pub fn get_directory_icon(expanded: bool) -> char {
    if expanded {
        '\u{f115}' //  Open folder
    } else {
        '\u{f114}' //  Closed folder
    }
}
