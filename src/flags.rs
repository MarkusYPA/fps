use crate::consts::DEFAULT_MAP_ID;

pub struct Flags {
    pub map_id: usize,
}

pub fn parse_flags<I>(args: I) -> Option<Flags>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    iter.next();

    let mut map_id = DEFAULT_MAP_ID;

    let args: Vec<String> = iter.collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-m" | "--map" => {
                if i + 1 < args.len() {
                    if let Ok(id) = args[i + 1].parse::<usize>() {
                        if id > 0 {
                            map_id = id;
                            i += 2;
                            continue;
                        }
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }

    Some(Flags { map_id })
}
