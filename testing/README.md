To run the exhaustive testing, simply use `cargo test`. It reads the file `output.txt` and checks it against `winarg`s parsing.

You can create a larger number of test cases but this requires some setup. First you'll need to build `args.exe`. From the testing directory run:

    cl src\args.cpp /nologo /O2 /link /MANIFEST:EMBED /MANIFESTINPUT:src\manifest.xml

Then you'll need to run the generator application:

    cargo run --release

This will overwrite `output.txt` with new test cases generated from running `args.exe` with different command lines. You can edit `main.rs` to increase or decrease the number to test cases produced.
