use std::fs::File;
use std::io::prelude::*;
use std::io::{Error};

/*
 デバッグ用に出力を制御するためのもの
 */
const PRINT_DEBAGU: bool = false;

const MAX_BUFFER_SIZE: usize = 1024;  // 1回の入力で受けつける最大のバイト
const MAX_MATCH_LEN: usize = 258;     // 最大でどれだけ一致するかのサイズ
const MIN_MATCH_LEN: usize = 3;       // 少なくとも３は一致しないと圧縮処理が行われない
const MAX_WINDOW_SIZE: usize = 1024;  // スライドウインドウの最大サイズ

/*
 bit単位で出力を行うためのもの
 bit_count:     bufferに何ビット突っ込んだかを保持する
 buffer:        出力用のbuffer
 output_vector: 出力データをこのvectorに溜めて最後に一気に出力する
 output:        出力ファイルデータ
 */
struct BitWriter<'a, T: Write> {
    bit_count: u8,
    buffer: u8,
    output_vector: Vec<u8>,
    output: &'a mut T,
}

impl<'a, T: Write> BitWriter<'a, T> {
    pub fn new(output: &'a mut T) -> Self {
        BitWriter {
            bit_count: 0,
            buffer: 0,
            output_vector: Vec::new(),
            output,
        }
    }

    /*
     deflate圧縮では出力方向が変わるため、ハフマン符号化したものや、距離符号のためのもの
     */
    pub fn code_bits(&mut self, bits: u16, bit_count: u8) -> Result<(), Error> {
        for i in 0..bit_count {
            if self.bit_count == 8 {
                self.flush_to_output()?;
            }
            let offset = bit_count - 1 - i;
            let bit = (bits & (1 << offset)) >> offset;
            self.buffer <<= 1;
            self.buffer |= bit as u8;
            self.bit_count += 1;
        }
        Ok(())
    }

    /*
     上以外のもの（拡張ビットや、ブロックの種類）
     */
    pub fn extra_bits(&mut self, bits: u16, bit_count: u8) -> Result<(), Error> {
        for i in 0..bit_count {
            if self.bit_count == 8 {
                self.flush_to_output()?;
            }
            let bit = (bits >> i) & 1;
            self.buffer <<= 1;
            self.buffer |= bit as u8;
            self.bit_count += 1;
        }
        Ok(())
    }

    /*
     最後にvecterに入っているものをまとめて出力する
     */
    pub fn flush(&mut self) -> Result<(), Error> {
        if self.bit_count > 0 {
            self.buffer <<= 8 - self.bit_count;
            let mut buffer = 0;
            for i in 0..8 {
                buffer <<= 1;
                buffer |= (self.buffer >> i) & 1;
            }

            self.output_vector.push(buffer.clone());
            if PRINT_DEBAGU == true {
                println!("push date: {:08b}", self.buffer);
                for i in 0..(self.output_vector.len()){
                    print!("{:08b}", self.output_vector[i]);
                }
                println!();
                println!("{:02x?}", self.output_vector);
            }
        }
        Ok(())
    }

    /*
     bufferが8ビット分溜まった時に実行される
     */
    fn flush_to_output(&mut self) -> Result<(), Error> {
        let mut buffer = 0;
        for i in 0..8 {
            buffer <<= 1;
            buffer |= (self.buffer >> i) & 1;
        }
        self.output_vector.push(buffer.clone());
        if PRINT_DEBAGU == true {
            println!("push date: {:08b}", buffer);
            for i in 0..(self.output_vector.len()){
                print!("{:08b}", self.output_vector[i]);
            }
            println!();
        }
        self.buffer = 0;
        self.bit_count = 0;
        Ok(())
    }
}

/*
 読み込みをbyteで保持するもの
 buffer:          データをMAX_BUFFER_SIZE分取り込むための配列。
 buf_count:       現在bufferが何個目まで読まれているかを保持する。
 buf_size:        bufferの何番目までデータがあるかを保持する
 flag:            読み込むデータがもうない時に使用する。
 file_size:       入力ファイルのサイズを記録する。
 input:           入力ファイルの情報を記録する。
 */
struct ByteReader<'a, T: Read> {
    buffer: [u8; MAX_BUFFER_SIZE],
    buf_count: usize,
    buf_size: usize,
    flag: bool,
    file_size: u32,
    input: &'a mut T,
}

impl<'a, T: Read> ByteReader<'a, T> {
    pub fn new(input: &'a mut T) -> Self {
        let mut reader = ByteReader {
            buffer: [0; MAX_BUFFER_SIZE],
            sub_buffer: 0,
            buf_count: 0,
            buf_size: 0,
            sub_buffer_flag: false,
            flag: true,
            file_size: 0,
            input,
        };
        let _ = reader.load_next_byte();
        reader
    }

    /*
     bufferが最後まで読まれたり、最初の読み込みの際に実行される。
     */
    fn load_next_byte(&mut self) -> Result<(), std::io::Error>{
        match self.input.read(&mut self.buffer)? {
            0 => self.flag = false,
            n => {
                self.file_size += n as u32;
                self.buf_size = n;
                self.flag = true;
            }
        };
        Ok(())
    }

    /*
     buf_countの位置にあるバイトを返す。
     */
    pub fn seek_byte(&mut self) -> u8{
        self.buffer[self.buf_count]
    }

    /*
     bit_countを進める。bufferの最後まできていた場合には
     load_next_byteで読み込む。
     */
    pub fn next_byte(&mut self) {
        if self.buf_count < MAX_BUFFER_SIZE &&
            self.buf_count < self.buf_size {
            self.buf_count += 1;
        } else {
            let _ = self.load_next_byte();
            self.buf_count = 0;
        }
    }

    /*
     bit_countの位置にあるバイトを返して、bit_countを進める。
     bufferの最後まできていた場合にはload_next_byteで読み込む。
     */
    pub fn get_byte(&mut self) -> u8 {
        if self.buf_count < MAX_BUFFER_SIZE &&
            self.buf_count < self.buf_size {
            let buffer = self.buffer[self.buf_count];
            self.buf_count += 1;
            buffer
        } else {
            let buffer = self.buffer[self.buf_count];
            let _ = self.load_next_byte();
            self.buf_count = 0;
            buffer
        }
    }
}

/*
 Crc32を計算するための構造体
 divisor:      除算を行う際に使用するbit列を保持する
 non_divisor:  除算される側のデータを保持する
 buffer:       とりあえずのデータを保持する
 buf_count:    bufferが何bit処理されたかを保持する
 first_count:  最初の4バイトは反転する必要があるためカウントする
 */
struct Crc32 {
    divisor: u32,
    non_divisor: u32,
    buffer: u8,
    buf_count: u8,
    first_count: u8,
}

impl Crc32 {
    pub fn new() -> Self {
        Crc32{
            divisor: 0b100110000010001110110110111,
            non_divisor: 0,
            buffer: 0,
            buf_count: 0,
            first_count: 0,
        }
    }

    /*
     non_divisorやbufferにデータを保持させるもの
     */
    pub fn push_buf(&mut self, buf: u8){
        let mut buffer: u8 = 0;
        for i in 0..8 {
            buffer <<= 1;
            buffer |= (buf >> i) & 1;
        }
        if self.first_count < 4 {
            self.non_divisor <<= 8;
            self.non_divisor |= !buffer as u32;
            self.first_count += 1;
        } else {
            self.buffer = buffer.clone();
            self.buf_count = 8;
            self.comp();
        }
    }

    /*
     先頭bitが立っている場合には除算を行い、それ以外の場合にはbufferのbitを先頭から突っ込む
     */
    fn comp(&mut self){
        for i in 0..self.buf_count{
            if self.non_divisor >= 2147483648{
                self.non_divisor <<= 1;
                self.non_divisor |= (((self.buffer as u16) >> (self.buf_count - i - 1)) & 1) as u32;
                self.comple();
            } else {
                self.non_divisor <<= 1;
                self.non_divisor |= (((self.buffer as u16) >> (self.buf_count - i - 1)) & 1) as u32;
            }
        }
        self.buf_count = 0
    } 

    /*
     除算を行う。実際にはxor
     */
    fn comple(&mut self){
        let buffer = self.non_divisor ^ self.divisor;
        self.non_divisor = buffer;
    }

    /*
     現在のnon_divisoreからcrc32を計算してそれを返す
     */
    fn get_crc32(&mut self) -> u32 {
        self.push_buf(0);
        self.push_buf(0);
        self.push_buf(0);
        self.push_buf(0);
        let mut buffer: u32 = 0;
        for i in 0..32 {
            buffer <<= 1;
            buffer |= (self.non_divisor >> i) & 1;
        }
        if PRINT_DEBAGU == true {
            println!("crc32: {:08x?}", !buffer);
        }
        !buffer
    }
}

/*
 zipのローカルヘッダーやセントラルヘッダー、エンドセントラルヘッダなどを
 保持するための構造体
 buffer: ヘッダー情報を保持する
 */
struct Header{
    buffer: Vec<u8>,
}

impl Header {
    pub fn new() -> Self{
        Header{
            buffer: Vec::new(),
        }
    }

    /*
     32bitの情報をbufferに追加する
     */
    fn push32(&mut self, num: u32) {
        let a = num & 0b11111111;
        let b = (num >> 8) & (0b11111111);
        let c = (num >> 16) & (0b11111111);
        let d = (num >> 24) & (0b11111111);
        self.buffer.push(a as u8);
        self.buffer.push(b as u8);
        self.buffer.push(c as u8);
        self.buffer.push(d as u8);
    }

    /*
    16bitの情報をbufferに追加する
     */
    fn push16(&mut self, num: u16) {
        let a = num & 0b11111111;
        let b = (num >> 8) & (0b11111111);
        self.buffer.push(a as u8);
        self.buffer.push(b as u8);
    }

    /*
     PK0506ヘッダであることを示す情報を追加する
     */
    fn push_pk0506(&mut self){
        self.buffer.push(0x50);
        self.buffer.push(0x4b);
        self.buffer.push(0x05);
        self.buffer.push(0x06);
    }

    /*
     PK0304ヘッダであることを示す情報を追加する
     */
    fn push_pk0304(&mut self){
        self.buffer.push(0x50);
        self.buffer.push(0x4b);
        self.buffer.push(0x03);
        self.buffer.push(0x04);
    }

    /*
     PK0102ヘッダであることを示す情報を追加する
     */
    fn push_pk0102(&mut self){
        self.buffer.push(0x50);
        self.buffer.push(0x4b);
        self.buffer.push(0x01);
        self.buffer.push(0x02);
    }

    /*
     ファイルの名前の情報を追加する
     */
    fn push_filename(&mut self, filename: &str) {
        let bytes: &[u8] = filename.as_bytes();
        for i in 0..bytes.len() {
            self.buffer.push(bytes[i]);
        }
    }

    /*
     ローカルヘッダーに必要な情報をもらって、ローカルヘッダーを作成する
     最終更新時間の情報が正しくできていないので最終更新時間を取得して書き換える
     */
    pub fn local_header(crc32: u32, before_size: u32, after_size: u32, filename: &str) -> Self {
        let mut header = Header::new();
        header.push_pk0304();
        header.push16(0x0014);
        header.push16(0x0000);
        header.push16(0x0008);
        header.push16(0x0000);
        header.push16(0x0000);
        header.push32(crc32);
        header.push32(after_size);
        header.push32(before_size);
        header.push16((filename.len()) as u16);
        header.push16(0x0000);
        header.push_filename(&filename);
        header
    }

    /*
     セントラルヘッダーに必要な情報をもらって、セントラルヘッダーを作成する
     最終更新時間の情報が正しくできていないので最終更新時間を取得して書き換える
     */
    pub fn central_header(crc32: u32, before_size: u32, after_size: u32, filename: &str) -> Self {
        let mut header = Header::new();
        header.push_pk0102();
        header.push16(0x0314);
        header.push16(0x0014);
        header.push16(0x0000);
        header.push16(0x0008);
        header.push16(0x0000);
        header.push16(0x0000);
        header.push32(crc32);
        header.push32(after_size);
        header.push32(before_size);
        header.push16((filename.len()) as u16);
        header.push16(0x0000);
        header.push16(0x0000);
        header.push16(0x0000);
        header.push16(0x0000);
        header.push32(0x00000000);
        header.push32(0x00000000);
        header.push_filename(&filename);
        header
    }

    /*
     エンドセントラルヘッダーに必要な情報をもらって、エンドセントラルヘッダーを作成する
     */
    pub fn end_header(header_size: u32, header_start: u32) -> Self {
        let mut header = Header::new();
        header.push_pk0506();
        header.push16(0x0000);
        header.push16(0x0000);
        header.push16(0x0001);
        header.push16(0x0001);
        header.push32(header_size);
        header.push32(header_start);
        header.push16(0x00);
        header
    }
}

/*
 windowの中にcheakと同じ並びのものがあるかを調べる。
 あった際には距離を返す。
 */
fn match_cheak<T: Eq>(window: &[T], cheak: &[T]) -> isize {
    if window.len() < cheak.len(){
        return -1;
    }
    'outer: for i in 0..(window.len() - cheak.len() + 1) {
        for j in 0..(cheak.len()){
            if window[i + j] != cheak[j]{
                continue 'outer;
            }
        }
        if PRINT_DEBAGU == true {
            println!("{} {} {}", window.len(), cheak.len(), i);
        }
        return (window.len() - cheak.len() - i + 1) as isize;
    }
    -1
}

/*
 固定ハフマンに変換する
 */
fn cheanger(num: usize) -> (u8, u16) {
    let (len, re) = match num {
        0   ..= 143 => (8, num + 0x30 ),
        144 ..= 255 => (9, num + 0x91 ),
        256 ..= 279 => (7, num - 0x100),
        280 ..= 287 => (8, num - 0x58 ),
        _ => (0, 512),
    };
    (len, re as u16)
}

/*
 長さから長さ符号と拡張ビットを調べる
 */
fn length_extra(date: u16) -> (u16, u8, u16){
    let (num, len, extra) = match date {
        3   ..=  10 => (date + 254, 0, 0),
        11  ..=  12 => (265, 1, ((date - 3)) & 0b1),
        13  ..=  14 => (266, 1, ((date - 3)) & 0b1),
        15  ..=  16 => (267, 1, ((date - 3)) & 0b1),
        17  ..=  18 => (268, 1, ((date - 3)) & 0b1),
        19  ..=  22 => (269, 2, ((date - 3)) & 0b11),
        23  ..=  26 => (270, 2, ((date - 3)) & 0b11),
        27  ..=  30 => (271, 2, ((date - 3)) & 0b11),
        31  ..=  34 => (272, 2, ((date - 3)) & 0b11),
        35  ..=  42 => (273, 3, ((date - 3)) & 0b111),
        43  ..=  50 => (274, 3, ((date - 3)) & 0b111),
        51  ..=  58 => (275, 3, ((date - 3)) & 0b111),
        59  ..=  66 => (276, 3, ((date - 3)) & 0b111),
        67  ..=  82 => (277, 4, ((date - 3)) & 0b1111),
        83  ..=  98 => (278, 4, ((date - 3)) & 0b1111),
        99  ..= 114 => (279, 4, ((date - 3)) & 0b1111),
        115 ..= 130 => (280, 4, ((date - 3)) & 0b1111),
        131 ..= 162 => (281, 5, ((date - 3)) & 0b11111),
        163 ..= 194 => (282, 5, ((date - 3)) & 0b11111),
        195 ..= 226 => (283, 5, ((date - 3)) & 0b11111),
        227 ..= 257 => (284, 5, ((date - 3)) & 0b11111),
        _ => (286, 6, 0)
    };
    (num as u16 ,len as u8 ,extra as u16)
}

/*
 距離から距離符号と拡張ビットを調べる
 */
fn distance_extra(date: u32) -> (u8, u8, u16){
    let (num, dis, extra) = match date {
        1     ..=     4 => (date - 1,0, 0),
        5     ..=     6 => (4 ,1 , (date - 1) & 0b1),
        7     ..=     8 => (5 ,1 , (date - 1) & 0b1),
        9     ..=    12 => (6 ,2 , (date - 1) & 0b11),
        13    ..=    16 => (7 ,2 , (date - 1) & 0b11),
        17    ..=    24 => (8 ,3 , (date - 1) & 0b111),
        25    ..=    32 => (9 ,3 , (date - 1) & 0b111),
        33    ..=    48 => (10,4 , (date - 1) & 0b1111),
        49    ..=    64 => (11,4 , (date - 1) & 0b1111),
        65    ..=    96 => (12,5 , (date - 1) & 0b11111),
        97    ..=   128 => (13,5 , (date - 1) & 0b11111),
        129   ..=   192 => (14,6 , (date - 1) & 0b111111),
        193   ..=   256 => (15,6 , (date - 1) & 0b111111),
        257   ..=   384 => (16,7 , (date - 1) & 0b1111111),
        385   ..=   512 => (17,7 , (date - 1) & 0b1111111),
        513   ..=   768 => (18,8 , (date - 1) & 0b11111111),
        769   ..=  1024 => (19,8 , (date - 1) & 0b11111111),
        1025  ..=  1536 => (20,9 , (date - 1) & 0b111111111),
        1537  ..=  2048 => (21,9 , (date - 1) & 0b111111111),
        2049  ..=  3072 => (22,10, (date - 1) & 0b1111111111),
        3073  ..=  4096 => (23,10, (date - 1) & 0b1111111111),
        4097  ..=  6144 => (24,11, (date - 1) & 0b11111111111),
        6145  ..=  8192 => (25,11, (date - 1) & 0b11111111111),
        8193  ..= 12288 => (26,12, (date - 1) & 0b111111111111),
        12289 ..= 16384 => (27,12, (date - 1) & 0b111111111111),
        16385 ..= 24576 => (28,13, (date - 1) & 0b1111111111111),
        24577 ..= 32768 => (29,13, (date - 1) & 0b1111111111111),
        _ => (31, 14, 0)
    };
    (num as u8 ,dis as u8, extra as u16)
}

/*
 エンコード処理を行い、zip形式で出力を行う。
 */
pub fn encode(input_file: &str, output_file: &str) -> Result<(), std::io::Error> {
    let mut input = File::open(input_file)?;
    let mut output = File::create(output_file)?;
    let mut input_reader = ByteReader::new(&mut input);
    let mut output_writer = BitWriter::new(&mut output);
    let mut crcs = Crc32::new();

    let mut window = Vec::new();

    output_writer.extra_bits(0b1, 1)?;
    output_writer.extra_bits(0b01, 2)?;

    let first = input_reader.get_byte();
    crcs.push_buf(first.clone());
    let (bit, first_date)= cheanger(first as usize);
    output_writer.code_bits(first_date, bit)?;

    loop{
        let byte = input_reader.get_byte();
        if PRINT_DEBAGU == true {
            println!("{:02x?}", byte);
        }
        if input_reader.flag == false { break;}
        crcs.push_buf(byte.clone());
        
        let mut res = vec![byte.clone()];

        let mut offset: isize = -1;

        window.push(res[0]);
        while res.len() < MAX_MATCH_LEN {
            let v = input_reader.seek_byte().clone();
            res.push(v);
            let new_offset = match_cheak(&mut window, &mut res);
            window.push(v);
            if new_offset == -1 {
                res.pop();
                window.pop();
                break;
            }
            offset = new_offset;
            crcs.push_buf(v.clone());
            input_reader.next_byte();
            if input_reader.flag == false { break };
        }
        if res.len() < MIN_MATCH_LEN {
            for byte in &res {
                let (bits, buf) = cheanger(*byte as usize);
                output_writer.code_bits(buf, bits)?;
                if PRINT_DEBAGU == true {
                    println!("{:09b} :{}", buf, bits);
                }
            }
        } else {
            let (num , data, extra) = length_extra(res.len() as u16);
            let (bits, buf) = cheanger(num as usize);
            output_writer.code_bits(buf, bits)?;
            if PRINT_DEBAGU == true {
                println!("{:09b} :{}", buf, bits);
            }
            output_writer.extra_bits(extra, data)?;
            if PRINT_DEBAGU == true {
                println!("{:05b} :{}", extra, data);
            }
            let (num , data, extra) = distance_extra(offset as u32);
            output_writer.code_bits(num as u16, 5)?;
            if PRINT_DEBAGU == true {
                println!("{:05b} :{}", num, 5);
            }
            output_writer.extra_bits(extra , data)?;
            if PRINT_DEBAGU == true {
                println!("{:09b} :{}", extra, data);
            }
        }
        if window.len() > MAX_WINDOW_SIZE{
            window.drain(0..(window.len() - MAX_WINDOW_SIZE));
        }

    }

    output_writer.code_bits(0b0000000, 7)?;
    output_writer.flush()?;

    let crc32 = crcs.get_crc32();
    let local_header = Header::local_header(crc32.clone(), input_reader.file_size, (output_writer.output_vector.len()) as u32, input_file);
    let central_header = Header::central_header(crc32.clone(), input_reader.file_size, (output_writer.output_vector.len()) as u32, input_file);
    let end_header = Header::end_header((central_header.buffer.len()) as u32, (local_header.buffer.len() + output_writer.output_vector.len()) as u32);

    if PRINT_DEBAGU == true {
        for i in 0..(output_writer.output_vector.len()){
            print!("{:08b}", output_writer.output_vector[i]);
        }
        println!();
    }

    output_writer.output.write_all(&local_header.buffer)?;
    output_writer.output.write_all(&output_writer.output_vector)?;
    output_writer.output.write_all(&central_header.buffer)?;
    output_writer.output.write_all(&end_header.buffer)?;

    Ok(())
}
