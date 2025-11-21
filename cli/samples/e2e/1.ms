workspace demo_ws {}

[entrypoint]
project test_pj {
    out = demo_stage(); // Should get the value from the initalized stage below
    i = (out + "!!!").length();
    say(i);
    say(out);
}

stage demo_stage () // No arguments
{
    arr = ["Hello", " ", "world"];
    out = "";
    for i in arr {
        out = out + i;
    }
    return out;
}