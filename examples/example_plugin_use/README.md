# Example 1: Hello workspace

The "Hello workspace" example demonstrates a simple workspace that outputs a greeting message.

## Script

```mainstage
workspace hello_ws {
    say("Hello workspace!");
}
```

## How to Run

1. Save the script above in a file named `script.ms` within the `examples/example_workspace/` directory.
2. Open your terminal and navigate to the `examples/example_workspace/` directory.
3. Run the script using the Mainstage command-line tool:

   ```bash
   mainstage run script.ms (options: --trace)
   ```

4. You should see the output:

   ```bash
   Hello workspace!
   ```

## Expected Output

```bash
Hello workspace!
```
