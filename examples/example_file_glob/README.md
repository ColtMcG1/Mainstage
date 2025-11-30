# Simple File Glob Example

This example demonstrates how to use file globbing to read multiple files in a workspace.

## Script

```ms
workspace file_glob_ws {
    files = glob(file_glob_project.sources);
    for (file in files) {
        content = read(file);
        say("Content of " + file + ": " + content);
    }
}

project file_glob_project {
    sources = "./data/*.txt";
}
```

## How to Run
1. Save the script above in a file named `script.ms` within the `examples/example_file_glob/` directory.
2. Create a `data` directory inside `examples/example_file_glob/` and add some `.txt` files with sample content.
3. Open your terminal and navigate to the `examples/example_file_glob/` directory.
4. Run the script using the command:
   ```
   mainstage run script.ms (options: --trace)
   ```

## Expected Output
```
Content of data/file1.txt: This is the content of file 1.
Content of data/file2.txt: This is the content of file 2.
...
```