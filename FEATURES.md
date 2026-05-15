- Each taskis made up of blocks
- A block is a single expression, or multiple expressions chained

- A @clean directive that removes a directory or a file
- A @stop directive that echos a string and stops the workflow
- An @echo directive that outputs to stdout
- I want @set to also have a block form where it sets a variable's value to the stdout of a another command. This form will only accept a single expression, but they can chained. An entire if-else block can be considered one expression if the contents have a single expression or @return is used. 
- A @return directive capturess the result of its expression, and passes it back to the 
- A @and conditionally runs a block or expression if the previous command returned 0 or true. Returns the result of the expression or false
- @or runs a block or expression if the previous command returned false or a number other than zero. Returns the result of the expression
- @success is an alias for @return true.
- @fail is an alias for @return false


- Allow semi-colons to work as a ine separator if thwo lines would run at the same level.

capture:
  @set MODE production
  @set RESULT:
    @task build; @and
      @if $MODE == production
        @return
          @shell ./scripts/deploy.sh
      @else
        @task clean-dist; @and
          @return "success"
        @or
          @return 1
  @and
    @return $RESULT
  @or
    @task fail-clean; @and
      @echo "sorry no tasks"
    @or
      @echo "I cannnot evenn get this right"


