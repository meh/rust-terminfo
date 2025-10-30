//            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
//                    Version 2, December 2004
//
// Copyleft (ↄ) meh. <meh@schizofreni.co> | http://meh.schizofreni.co
//
// Everyone is permitted to copy and distribute verbatim or modified
// copies of this license document, and changing it is allowed as long
// as the name is changed.
//
//            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
//   TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION
//
//  0. You just DO WHAT THE FUCK YOU WANT TO.

#[inline]
pub fn number(i: &[u8]) -> i32 {
	let mut n: i32 = 0;

	for &ch in i {
		let d = (ch as i32).wrapping_sub(b'0' as i32);

		if d <= 9 {
			n = n.saturating_mul(10).saturating_add(d);
		}
	}

	n
}
