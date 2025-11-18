workspace demo_ws {}

[entrypoint]
project test_pj {
    demo_stage(); // Should get the value from the initalized stage below
}

stage demo_stage () // No arguments
{
    for i=0 to 2 {
        say("Iteration " + i);
    }
}