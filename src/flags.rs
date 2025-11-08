use crate::consts::DEFAULT_MAP_ID;

#[derive(Debug, Clone)]
pub enum MapIdentifier {
    Id(usize),
    Name(String),
}

pub struct Flags {
    pub map: MapIdentifier,
}

pub fn parse_flags<I>(args: I) -> Option<Flags>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    iter.next();

    let mut map = MapIdentifier::Id(DEFAULT_MAP_ID);

    let args: Vec<String> = iter.collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-m" | "--map" => {
                if i + 1 < args.len() {
                    let value = &args[i + 1];
                    // Try parsing as usize first
                    if let Ok(id) = value.parse::<usize>() {
                        if id > 0 {
                            map = MapIdentifier::Id(id);
                            i += 2;
                            continue;
                        }
                    }
                    // If parsing as usize fails, treat it as a map name
                    map = MapIdentifier::Name(value.clone());
                    i += 2;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }

    Some(Flags { map })
}
