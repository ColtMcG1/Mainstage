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