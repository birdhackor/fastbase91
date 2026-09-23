#include "vendor/base91-0.6.0/base91.h"

#include <stdio.h>
#include <string.h>

enum operation {
	ENCODE,
	DECODE
};

int main(int argc, char **argv)
{
	unsigned char input[4096];
	unsigned char output[8194];
	struct basE91 state;
	enum operation operation;
	size_t input_len;

	if (argc != 2) {
		fprintf(stderr, "usage: %s encode|decode\n", argv[0]);
		return 2;
	}
	if (strcmp(argv[1], "encode") == 0)
		operation = ENCODE;
	else if (strcmp(argv[1], "decode") == 0)
		operation = DECODE;
	else {
		fprintf(stderr, "unknown operation: %s\n", argv[1]);
		return 2;
	}

	basE91_init(&state);
	while ((input_len = fread(input, 1, sizeof(input), stdin)) != 0) {
		size_t output_len;

		if (operation == ENCODE)
			output_len = basE91_encode(&state, input, input_len, output);
		else
			output_len = basE91_decode(&state, input, input_len, output);
		if (fwrite(output, 1, output_len, stdout) != output_len)
			return 1;
	}
	if (ferror(stdin))
		return 1;

	{
		size_t output_len;

		if (operation == ENCODE)
			output_len = basE91_encode_end(&state, output);
		else
			output_len = basE91_decode_end(&state, output);
		if (fwrite(output, 1, output_len, stdout) != output_len)
			return 1;
	}

	return fflush(stdout) == 0 ? 0 : 1;
}
