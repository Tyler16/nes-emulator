NUM_CHARACTERS = 73

input_file_path = "nestest.log"
output_file_path = "modifiedtest.log"

with open(input_file_path, 'r') as input_file, open(output_file_path, 'w') as output_file:
    for line in input_file:
        truncated_line = line[:NUM_CHARACTERS]
        output_file.write(truncated_line + '\n')