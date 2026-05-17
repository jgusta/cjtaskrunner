
- new directives: `@mkdir` which functions as cross-platform `mkdir -p`. `@cpdir` which copies one or more directories to another location using relative path (from the cwd if any `@cd` has been used) using standard unix-like trailing slash rules; and `@cp` which copies one or more files to another location. `@rename` can rename files and directories but not move them. 
- Safety concern: due to the `cj folder task` it should be enforced that tasks cannnot be named the same as a folder.
- implement inifinite loop detection
