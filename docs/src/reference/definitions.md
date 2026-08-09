# Definitions

### Environment
> State of your computer from which commands are executed.
- consists of the name of your current directory, your current user, its permissions, the [variables](#variable) accessible to it and the commands it can run at the moment.

### Environment variable
> A variable that can be read by executables and CJTaskrunner.
- Set in your shell before CJTaskrunner runs, or 
- Set using the [`@env`](directives.md#env) directive.
- Globally available within a [taskfile execution context](#taskfile-execution-context).

### Runtime variable
> A variable whose value can be read only by CJTaskrunner inside of a task.
- Set in a task using the [`@set`](directives.md#set) directive.
- Accessible only in CJTaskrunner.
- Lasts the duration of the **[task context](#task-context)** and is passed to subordinate tasks as a snapshot.
- Will override other [variables](#variable) with the same name for the duration of the task. Most recent definition wins.
- Can be promoted to an [exported variable](#exported-variable).

### Exported variable
> A variable that has the same properties of an [environment variable](#environment-variable) but originated as a [runtime variable](#runtime-variable).
- A [runtime variable](#runtime-variable) that is used in an [`@export`](directives.md#export) directive.
- As a shortcut can be defined directly in the [`@export`](directives.md#export) directive.
- Exported values are shared across the [taskfile execution context](#taskfile-execution-context). A task can still shadow the same name with [`@set`](directives.md#set) inside its own [task context](#task-context), or overwrite the shared value with another [`@export`](directives.md#export).

### Task context
> The [environment](#environment) that spans the execution of a single task. 
- This is the narrowest and most isolated context.
- It starts as a copy of the **[taskfile execution context](#taskfile-execution-context)** from which the task is run. 
- A copy of it is propagated to subordinate task runs using [`@task`](directives.md#task) and [`@await`](directives.md#await).
- Unexported variable changes are discarded when the task ends, as well as working directory changes.

### Taskfile execution context
> the **[environment](#environment)** that exists from the moment you run a CJTaskrunner task until it finishes. 

- Starting as a snapshot of your shell [environment](#environment), it holds any **[environment variable](#environment-variable)** references and **[exported variables](#exported-variable)** and makes them available to **[task contexts](#task-context)**.
- Ultimately, all environment changes made in a [taskfile execution context](#taskfile-execution-context) are discarded after CJTaskrunner completes its run.


### Variable
> A named container holding a value.
- You refer to a variable by its name, such as MR_VAR.
- Variables are further divided into [runtime variables](#runtime-variable), [environment variables](#environment-variable), and [exported variables](#exported-variable).
- The name must consist of ascii letters, digits or an underscore. It cannot start with a digit.
- If you put a `$` in front of the variable name, CJTaskrunner will exchange the whole thing for the variable's value. This is what is called "[interpolating](variables.md#interpolation)" or "dereferencing".
