# jmrg

`jmrg` is a command line utility that allows you to merge multiple sorted [NDJSON](http://ndjson.org/)
(Newline delimited JSON) files into a single stream.

This can be useful for combining multiple data
sources or processing large datasets that have been split into smaller files.
The best example of this is merging of multiple log files.

It uses [infer](https://docs.rs/infer/latest/infer/) to determine the file extension and can handle plain text, 
gzip-compressed and bzip2-compressed files.

## Installation

The pre-built binaries can be found on the [releases page in GitHub](https://github.com/maxfilov/jmrg/releases).

If you want to build it yourself, you can use [cargo](https://github.com/rust-lang/cargo):
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

If utility can not find any of the specified keys, it omits the entry completely.
By default, there are only one key: `"timestamp"`.

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
