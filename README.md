# jmrg

`jmrg` is a command line utility that allows you to merge multiple sorted [NDJSON](http://ndjson.org/)
(Newline delimited JSON) files into a single stream.

This can be useful for combining multiple data
sources or processing large datasets that have been split into smaller files.
The main reason I created this software in the first place was to merge multiple log files from different services.

## Installation

You can install `jmrg` using cargo:
```shell
cargo install --git https://github.com/maxfilov/jmrg
```

## Usage

To use `jmrg`, specify the input files as command line arguments and redirect
the output to a file or another command. For example:
```shell
jmrg input1.ndjson input2.ndjson > output.ndjson
```
This will merge the contents of `input1.ndjson` and `input2.ndjson` into a single sorted stream and write it to `output.ndjson`.

You can also pipe the output of `jmrg` to another command for further processing:
```shell
jmrg input1.ndjson input2.ndjson | jq '.timestamp' > output.txt
```

### Command line options

`jmrg` supports the following options:

- `-k <field>`: specify the field to use for sorting, can be specified multiple times (default: 'timestamp')
- `-h,--help`: display help information and exit

## Contributing

If you find a bug or have an idea for a new feature, feel free to open an issue or submit a pull request on the
[GitHub repository](https://github.com/maxfilov/jmrg). We welcome contributions from everyone.

## License

`jmrg` is released under the [MIT License](https://opensource.org/licenses/MIT).
See the [LICENSE](./LICENSE) file for more details.
