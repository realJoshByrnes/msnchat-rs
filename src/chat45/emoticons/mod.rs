pub mod patch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmoticonMatch {
    pub id: u32,
    pub length: usize,
}

/// Parses an emoticon from the start of the `input` string.
/// This is the Rust counterpart of `sub_372119EE`.
/// Returns the resource ID and the length of the emoticon string.
// Note added 09/03/2026 ~ JD
// TODO: We don't need the length, we can just use the string length. This is just how the original worked.
pub fn parse_emoticon(input: &str) -> Option<EmoticonMatch> {
    let mut chars = input.chars();
    let prefix = chars.next()?;

    match prefix {
        ':' => {
            const TABLE: &[(&str, u32, usize)] = &[
                (":)", 111, 2),
                (":-)", 111, 3),
                (":S", 116, 2),
                (":s", 116, 2),
                (":-S", 116, 3),
                (":-s", 116, 3),
                (":|", 113, 2),
                (":-|", 113, 3),
                (":(", 112, 2),
                (":-(", 112, 3),
                (":<", 112, 2),
                (":-<", 112, 3),
                (":D", 117, 2),
                (":d", 117, 2),
                (":-D", 117, 3),
                (":-d", 117, 3),
                (":>", 117, 2),
                (":->", 117, 3),
                (":-O", 119, 3),
                (":-o", 119, 3),
                (":O", 119, 2),
                (":o", 119, 2),
                (":P", 118, 2),
                (":p", 118, 2),
                (":-P", 118, 3),
                (":-p", 118, 3),
                (":[", 127, 2),
                (":-[", 127, 3),
                (":'(", 144, 3),
                (":@", 147, 2),
                (":-@", 147, 3),
                (":$", 155, 2),
                (":-$", 155, 3),
                (":-#", 158, 3),
                (":-*", 159, 3),
                (":^)", 160, 3),
            ];
            find_match(input, TABLE)
        }
        '(' => {
            const TABLE: &[(&str, u32, usize)] = &[
                ("(A)", 150, 3),
                ("(a)", 150, 3),
                ("(al)", 190, 4),
                ("(ap)", 169, 4),
                ("(au)", 170, 4),
                ("(B)", 114, 3),
                ("(b)", 114, 3),
                ("(bah)", 171, 5),
                ("(brb)", 172, 5),
                ("(C)", 142, 3),
                ("(c)", 142, 3),
                ("(ci)", 173, 4),
                ("(co)", 174, 4),
                ("(D)", 120, 3),
                ("(d)", 120, 3),
                ("(E)", 122, 3),
                ("(e)", 122, 3),
                ("(F)", 123, 3),
                ("(f)", 123, 3),
                ("(G)", 121, 3),
                ("(g)", 121, 3),
                ("(H)", 137, 3),
                ("(h)", 137, 3),
                ("(h5)", 175, 4),
                ("(I)", 136, 3),
                ("(i)", 136, 3),
                ("(ip)", 176, 4),
                ("(K)", 124, 3),
                ("(k)", 124, 3),
                ("(L)", 126, 3),
                ("(l)", 126, 3),
                ("(li)", 177, 4),
                ("(M)", 125, 3),
                ("(m)", 125, 3),
                ("(mo)", 178, 4),
                ("(mp)", 179, 4),
                ("(N)", 128, 3),
                ("(n)", 128, 3),
                ("(O)", 152, 3),
                ("(o)", 152, 3),
                ("(0)", 152, 3),
                ("(P)", 132, 3),
                ("(p)", 132, 3),
                ("(pi)", 180, 4),
                ("(pl)", 181, 4),
                ("(S)", 139, 3),
                ("(sn)", 182, 4),
                ("(so)", 183, 4),
                ("(st)", 184, 4),
                ("(R)", 156, 3),
                ("(r)", 156, 3),
                ("(T)", 131, 3),
                ("(t)", 131, 3),
                ("(tu)", 185, 4),
                ("(U)", 115, 3),
                ("(u)", 115, 3),
                ("(um)", 186, 4),
                ("(W)", 143, 3),
                ("(w)", 143, 3),
                ("(X)", 133, 3),
                ("(x)", 133, 3),
                ("(xx)", 187, 4),
                ("(Y)", 129, 3),
                ("(y)", 129, 3),
                ("(yn)", 188, 4),
                ("(Z)", 134, 3),
                ("(z)", 134, 3),
                ("(%)", 135, 3),
                ("(~)", 154, 3),
                ("(*)", 140, 3),
                ("(8)", 138, 3),
                ("(6)", 151, 3),
                ("(@)", 141, 3),
                ("(#)", 157, 3),
                ("(&)", 153, 3),
                ("(^)", 145, 3),
                ("({)", 148, 3),
                ("(})", 149, 3),
                ("(?)", 146, 3),
                ("(||)", 189, 4),
            ];
            find_match(input, TABLE)
        }
        '*' => {
            const TABLE: &[(&str, u32, usize)] = &[("*-)", 165, 3)];
            find_match(input, TABLE)
        }
        '+' => {
            const TABLE: &[(&str, u32, usize)] = &[("+o(", 166, 3)];
            find_match(input, TABLE)
        }
        '8' => {
            const TABLE: &[(&str, u32, usize)] =
                &[("8o|", 161, 3), ("8-)", 162, 3), ("8-|", 163, 3)];
            find_match(input, TABLE)
        }
        ';' => {
            const TABLE: &[(&str, u32, usize)] = &[(";)", 130, 2), (";-)", 130, 3)];
            find_match(input, TABLE)
        }
        '<' => {
            const TABLE: &[(&str, u32, usize)] = &[("<:o)", 168, 4)];
            find_match(input, TABLE)
        }
        '|' => {
            const TABLE: &[(&str, u32, usize)] = &[("|-)", 167, 3)];
            find_match(input, TABLE)
        }
        '^' => {
            const TABLE: &[(&str, u32, usize)] = &[("^o)", 164, 3)];
            find_match(input, TABLE)
        }
        _ => None,
    }
}

fn find_match(input: &str, table: &[(&str, u32, usize)]) -> Option<EmoticonMatch> {
    for &(s, id, len) in table {
        // We match exactly the emoticon string at the beginning of the input
        if input.starts_with(s) {
            return Some(EmoticonMatch { id, length: len });
        }
    }
    None
}
