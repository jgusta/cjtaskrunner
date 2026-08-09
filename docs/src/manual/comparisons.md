# Comparisons

Task runners are often tied to a language ecosystem or designed around build
artifacts. CJTaskrunner instead provides a small, project-local catalog of
imperative commands.

## `make`

- Target-based
- Makefiles are declarative at the dependency-graph level
- Designed for long build processes
- Focused on orchestrating compiled artifacts
- Doesn't list tasks or document them easily
- Values idempotency and deterministic results
- Arbitrary tasks are secondary

## `rake`

- Target-based
- Uses Ruby syntax and language constructs
- Can be used outside Ruby projects
- Lists tasks, which inspired CJTaskrunner's summary mode
- Can be used as an imperative task runner

## Apache Ant

- Target-based
- Uses a verbose XML structure
- Describes tasks with quoted XML attributes
- Lots of syntax to learn
- Lots of boilerplate and typing
- Requires Java

## Vite

- Task- and target-based
- Provides built-in features and plugins for web development
- Uses JavaScript configuration
- Requires Node.js

## Conclusion

CJTaskrunner does not replace these tools. It provides a directory and
launch point for them, while remaining sufficient for simple workflows on its
own.
