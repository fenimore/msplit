# msplit

A utility for splitting mp3 files into smaller partitions losslessly.

MP3 files are compressed audio divided into *frames*. `msplit` is lossless
because instead of decoding and reencoding the audio file, it splits the already
compressed frames.


```
msplit.

Usage:
  msplit <filename>
  msplit <filename> [--seconds=number] [--output=filename] [--dir=dirname]
  msplit <filename> [-s number] [-o filename] [-d dirname]

Options:
  -h --help     Show this screen.
  -s --seconds=number  Duration of partition [default: 10].
  -o --output=filename Partition filename prefix [default: partition]
  -d --dir=dirname  Output directory (created if it does not already exist) [default: partitions].
```
