# Example Stage

This directory contains an example stage. It demonstrates how to set up a basic stage with various elements and interactions.

## Script

```mainstage
workspace mainstage_example
{
    files = example_stage("data/*.txt");
    for (file in files)
    {
        process_file(file);
    }
}

stage example_stage(path)
{
    files = read(path);
    return files;
}

stage process_file(file)
{
    say("-------------------------");
    say(content);
}
```

## How to Run

1. Save the script above in a file named `script.ms` within the `examples/example_stage/` directory.
2. Create a `data` directory inside `examples/example_stage/` and add some `.txt` files with sample content.
3. Open your terminal and navigate to the `examples/example_stage/` directory.
4. Run the script using the command: `mainstage run script.ms`.
5. Observe the output displaying the contents of each `.txt` file in the `data` directory.

## Expected Output

```bash
-------------------------
Content of file1.txt
-------------------------
Content of file2.txt
...
```
