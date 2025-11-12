workspace demo_ws {
    test = [1,2,3]             // Define an array with three elements: 1,2,3
    index1 = test[2] - test[1] // 3 - 2 = 1
    say(test[0])               // Write "1" to console
    say(test[index1])          // Write "2" to console
    say(test[2])               // Write "3" to console
}