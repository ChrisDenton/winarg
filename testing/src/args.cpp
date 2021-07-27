// To build:
// cl src\args.cpp /nologo /O2 /link /MANIFEST:EMBED /MANIFESTINPUT:src\manifest.xml
//
// TODO: Use `cc` in a build script to build this automatically.
#include <Windows.h>
#include <stdio.h>

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