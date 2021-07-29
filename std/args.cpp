// To build:
// cl src\args.cpp /nologo /MT /O2 /link /MANIFEST:EMBED /MANIFESTINPUT:src\manifest.xml
#include <Windows.h>
#include <stdio.h>

// Output the unparsed command line, the argument count and then each parsed argument.
// Use `\0` as the separator instead of a new line if you don't mind the output being non-text.
int main(int argc, char *argv[]) {
	char num_args[10];
	_itoa_s(argc, num_args, sizeof(num_args), 10);

	puts(GetCommandLineA());
	puts(num_args);
	for (int i = 0; i < argc; i++) {
		puts(argv[i]);
	}
	return 0;
}