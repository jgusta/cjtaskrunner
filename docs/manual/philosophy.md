# Philosophy

Core principle: "never be a problem."

With this principle in mind, these are the goals of this project...


## CJTaskrunner

- Should reduce cognitive load.
- Should minimize typing without hiding behavior.
- Should work with existing tools instead of replacing them.
- Should never do anything that cannot be done without it.
- Should always be optional.
- Should have a discoverable interface.
- Should be strict in its syntax, but
- Should tell you how to fix errors when they happen.
- Should run tasks as if the user is running them directly.


## Taskfiles
- Are minimal; basic syntax conveyable in one sentence.
- Are imperative; describing workflows rather than build targets.
- Are readable; syntax should be understandable to non-users.
- Are shallow; maximum nesting of one level.
- Are descriptive; descriptions are easy to add and easy to find.
- Are predictable; a taskfile's directory is its project root.
- Are catalog-like; serve as a repository's catalog of useful commands.


## Directives

- Are limited; they do one thing.
- Are optional; they are not needed to simply run tasks.
- Are useful; they only exist where they can save time.
- Are explicit; they are the first thing on their line and never expand into values.
- Are minimal; they accept names, paths, or user-facing text instead of options.
- Are flat; they have longer names instead of options of a broader directive.
