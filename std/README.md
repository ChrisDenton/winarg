This branch is a very minimal testing framework to make it easier for future people to reproduce test cases for Windows argument parsing. There are three programs here.

Firstly a C++ program, `args.cpp`, that output its raw commandline, `argc` and then each argument in `argv`:

    cl src\args.cpp /nologo /MT /O2 /link /MANIFEST:EMBED /MANIFESTINPUT:src\manifest.xml

`gen_test_Cases.rs` repeatedly calls `args.exe` with different command line arguments and saves the output to a file.

`gen_test_cases.rs` transforms the ouput of the test cases into a form that can be used in `library\std\src\sys\windows\args\tests.rs`. This only needs to be used when updating standard library tests.
