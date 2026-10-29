# CJTaskrunner is not a shell

CJTaskrunner is not a shell, nor does it require a shell to run.

You cannot use shell-specific syntax in your task definitions in your `cjtasks` files. This includes:

- "globbing" - not supported by CJTaskrunner. Wildcard matching such as using the asterisk character (`*`) to indicate all the files in a directory or a question mark (`?`) to match a single character in a file name. 
- "brace expansion" or "ranges" - not supported by CJTaskrunner. Such as `a{1,2,3}` or `{1..3}`
- "subshells" - not supported by CJTaskrunner. Running commands in an isolated process by using parentheses `(cd /tmp && touch file.txt)`
- "command substitution" - not supported by CJTaskrunner. Running a command and injecting its output in a command or string `current_dir=$(pwd)`
- "inline variable assignment" - setting an environment variable for the duration of a command `NODE_ENV=production node app.js`. 

CJTaskrunner can do some of these things using directives, however.

- Use `@export` to change an enviroment variable before running a command

- Capture the stdout of a command using `@set:`

The irony of the statement at the top of this page is that, like a shell, CJTaskrunner runs executables directly, supplying arguments. So you can always use `env` in your task if you really need to do the inline syntax:

```cjtasks
mytask:
  env NODE_ENV=production node app.js
```

CJTaskrunner can run any commands that are discrete executables. Much of what you might think of as `bash` syntax may actually be a program.

And finally if you REALLY need to, you can use `@shell` to run things in your system shell. This will void your CJTaskrunner warranty however.

