# Rust-zipper

このプログラムは、Rust言語でzip圧縮を行うためのものとなっている。
方法としては、Deflate圧縮の固定ハフマン方式を採用している。

現在は一つのファイルを圧縮することしかできない。
（フォルダを指定して圧縮を行えない。）

不具合
今回はテキストファイルを対象としているため、他のpngやpdfなどを圧縮して解凍した際に展開することができない。
文字列の中にアスキーコード以外の文字が含まれていると（日本語など）展開することができなかったり、展開できても
中身が空っぽのファイルになってしまったりといったものがある


参考にしたサイト

crc32の実装
https://www.slideshare.net/7shi/crc32

固定ハフマン
https://darkcrowcorvus.hatenablog.jp/?page=1483525541
https://wiki.suikawiki.org/n/DEFLATE#anchor-106
https://www.slideshare.net/7shi/deflate

zipのフォーマット
https://hgotoh.jp/wiki/doku.php/documents/other/other-017
http://menyukko.ifdef.jp/cauldron/dtzipformat.html