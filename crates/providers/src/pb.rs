#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Val<'a> {
    Varint(u64),
    Fixed64(u64),
    Bytes(&'a [u8]),
    Fixed32(u32),
}

fn varint(b: &[u8], i: &mut usize) -> Option<u64> {
    let mut out = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = *b.get(*i)?;
        *i += 1;
        out |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(out);
        }
    }
    None
}

pub fn parse(b: &[u8]) -> Option<Vec<(u32, Val<'_>)>> {
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let tag = varint(b, &mut i)?;
        let field = u32::try_from(tag >> 3).ok().filter(|f| *f > 0)?;
        let v = match tag & 7 {
            0 => Val::Varint(varint(b, &mut i)?),
            1 => {
                let s = b.get(i..i + 8)?;
                i += 8;
                Val::Fixed64(u64::from_le_bytes(s.try_into().ok()?))
            }
            2 => {
                let len = usize::try_from(varint(b, &mut i)?).ok()?;
                let s = b.get(i..i.checked_add(len)?)?;
                i += len;
                Val::Bytes(s)
            }
            5 => {
                let s = b.get(i..i + 4)?;
                i += 4;
                Val::Fixed32(u32::from_le_bytes(s.try_into().ok()?))
            }
            _ => return None,
        };
        out.push((field, v));
    }
    Some(out)
}

pub fn text<'a>(v: &Val<'a>) -> Option<&'a str> {
    match v {
        Val::Bytes(b) => std::str::from_utf8(b).ok().filter(|s| !s.is_empty() && s.chars().all(|c| !c.is_control())),
        _ => None,
    }
}

pub fn get<'a, 'b>(fields: &'b [(u32, Val<'a>)], n: u32) -> Option<&'b Val<'a>> {
    fields.iter().find(|(f, _)| *f == n).map(|(_, v)| v)
}

pub type Fields<'a> = [(u32, Val<'a>)];

pub fn walk<'a>(b: &'a [u8], depth: usize, visit: &mut dyn FnMut(&Fields<'a>)) {
    if depth == 0 {
        return;
    }
    let Some(fields) = parse(b) else { return };
    visit(&fields);
    for (_, v) in &fields {
        if let Val::Bytes(inner) = v {
            walk(inner, depth - 1, visit);
        }
    }
}

#[cfg(test)]
pub mod build {
    pub fn varint(mut v: u64, out: &mut Vec<u8>) {
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                out.push(b);
                return;
            }
            out.push(b | 0x80);
        }
    }
    pub fn bytes(field: u32, data: &[u8], out: &mut Vec<u8>) {
        varint(u64::from(field) << 3 | 2, out);
        varint(data.len() as u64, out);
        out.extend_from_slice(data);
    }
    pub fn uint(field: u32, v: u64, out: &mut Vec<u8>) {
        varint(u64::from(field) << 3, out);
        varint(v, out);
    }
    pub fn f32(field: u32, v: f32, out: &mut Vec<u8>) {
        varint(u64::from(field) << 3 | 5, out);
        out.extend_from_slice(&v.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_rejects_garbage() {
        let mut m = Vec::new();
        build::uint(1, 300, &mut m);
        build::bytes(2, b"hello", &mut m);
        build::f32(3, 0.5, &mut m);
        let f = parse(&m).unwrap();
        assert_eq!(get(&f, 1), Some(&Val::Varint(300)));
        assert_eq!(text(get(&f, 2).unwrap()), Some("hello"));
        assert_eq!(get(&f, 3), Some(&Val::Fixed32(0.5f32.to_bits())));
        assert!(parse(&[0x0a, 0x05, 0x01]).is_none());
        assert!(parse(&[0x07]).is_none());
    }
}
