use axiom_l1_rs::stream::ApkParser;
use axiom_l1_rs::event::ParseEvent;
use axiom_zip_ref::eocd;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "/root/apkaxiom/corpus/whatsapp/whatsapp-latest.apk".into());
    let bytes = std::fs::read(&path).unwrap();
    println!("path: {}", path);
    println!("file size: {}", bytes.len());

    let real_eocd = eocd::find_eocd(&bytes);
    println!("real EOCD offset (backward scan over full bytes): {:?}", real_eocd);

    let cur = std::io::Cursor::new(&bytes);
    let mut parser = ApkParser::from_reader(cur);
    let mut events = 0u64;
    let mut last_header_name: String = String::new();
    let mut header_count = 0u64;
    let mut data_chunks = 0u64;
    let mut data_bytes = 0u64;
    let mut last_offset: u64 = 0;

    loop {
        match parser.next_event() {
            Ok(Some(ev)) => {
                events += 1;
                match ev {
                    ParseEvent::ZipEntryHeader { file_name, offset, compressed_size, .. } => {
                        header_count += 1;
                        last_offset = offset;
                        if header_count <= 3 || header_count % 1000 == 0 {
                            println!("  [{:>5}] LFH @ {:>10}  csize={:>10}  name={}",
                                header_count, offset, compressed_size,
                                String::from_utf8_lossy(&file_name));
                        }
                        last_header_name = String::from_utf8_lossy(&file_name).to_string();
                    }
                    ParseEvent::ZipEntryData { bytes, .. } => {
                        data_chunks += 1;
                        data_bytes += bytes.len() as u64;
                    }
                    ParseEvent::EocdSeen { offset, total_entries, .. } => {
                        println!("EOCD reached @ {}  total_entries={}", offset, total_entries);
                    }
                    ParseEvent::ParseComplete { entries, bytes } => {
                        println!("ParseComplete: entries={}  bytes={}", entries, bytes);
                    }
                    _ => {}
                }
            }
            Ok(None) => {
                println!("\nEND.  events={}  headers={}  data_chunks={}  data_bytes={}",
                         events, header_count, data_chunks, data_bytes);
                println!("last LFH @ stream-offset {}  ({})", last_offset, last_header_name);
                break;
            }
            Err(e) => {
                println!("\nERROR after events={}  headers={}  data_chunks={}  data_bytes={}",
                         events, header_count, data_chunks, data_bytes);
                println!("last LFH @ stream-offset {}  ({})", last_offset, last_header_name);
                println!("ERROR: {}", e);
                break;
            }
        }
    }
}
