use anyhow::{Result as AResult, Ok as AOk, Context};
use ocl::{ProQue};
use crate::backend::Rule;

pub struct Gpu {
	pro_que: ProQue,
	wg_size: usize
}

pub fn try_setup_gpu() -> AResult<Option<Gpu>> {
	let src = r#"
		__constant const unsigned AS[96] = {1121899819, 630111683, 1501065279, 458365203, 969558243, 1876681249, 962194431, 1077359051, 265679591, 791886952, 1582116761, 1676571504, 1476289907, 1117239683, 1503178135, 1341148412, 902714229, 1331438416, 58133212, 831516153, 285337308, 526856546, 362935496, 750214563, 210465667, 1381224997, 1846331200, 1330597961, 593162892, 1729496551, 792803163, 565661843, 863554642, 53838754, 749855384, 93067682, 1778866589, 1463507567, 367760674, 1219347826, 1648614489, 596622148, 1228675679, 243017841, 1132230640, 1891159862, 730619752, 33642253, 209795643, 283831563, 249493290, 967871855, 1560699908, 437500212, 429989927, 595043273, 267176187, 750850716, 247899536, 1735816893, 30350049, 1779080117, 484196159, 683666687, 2146567436, 1016454918, 813016862, 1422451153, 367384299, 1410110453, 1709765470, 1586690309, 963677742, 986269033, 1330385311, 1836198807, 1445664514, 119917655, 1765467570, 466789452, 650605245, 1812688947, 1120802318, 309331329, 1480003261, 1972414955, 1152445582, 426054430, 1771332474, 154812111, 1973375142, 1028015873, 1215608031, 779427428, 1188997777, 2017018019};
		__constant const unsigned BS[96] = {1559595546, 1755192844, 1649316166, 1198642031, 442452829, 1200195957, 1945678308, 949569752, 2099272109, 587775847, 626863973, 1003550677, 1358625013, 1008269081, 2109153755, 65212616, 1851925803, 2137491580, 1454235444, 675580731, 1754296375, 1821177336, 2130093701, 70062080, 1503113964, 1130186590, 2005789796, 1476653312, 1174277203, 174182291, 401846963, 973512717, 638171722, 2122881600, 1380182313, 1638451829, 65271247, 818200948, 736891500, 2056119311, 1084756724, 1537539262, 255459778, 587232589, 1947978014, 1706746116, 724046315, 981848395, 315304373, 475269784, 880625662, 1543454120, 1331075398, 1047903413, 418573418, 1885901857, 1772582790, 1579254086, 1843011714, 1459749886, 1341889808, 469024996, 1922776196, 1925089818, 185928884, 1800834903, 365378955, 1383227060, 1775570415, 470701926, 2147425016, 1033724855, 1400600080, 1545599780, 1738307654, 216757113, 1565717558, 1542861112, 269567713, 1943851495, 406140275, 1023941401, 1161348939, 699007419, 1441040276, 1005876490, 1789920966, 1737751956, 1704308182, 1641764103, 2013352686, 633500808, 1122672881, 1424625261, 714229503, 615731728};
		__kernel void gather_matching_seeds(
			__global unsigned *out,
			const unsigned char rush_len,
			const unsigned char subset_rules_len,
			const unsigned short sequence_rules_size,
			__global const unsigned char *subset_rules,
			__global const unsigned char *sequence_rules
		){
			const unsigned r = get_global_id(0);
			for(unsigned i = 0; i < 16; ++i){
				out[(unsigned)16*r + i] = ~(unsigned)0;
			}
			const unsigned w = get_global_size(0);
			const unsigned S_LO_BOUND = 2147483648u/w;
			const unsigned s_hi = S_LO_BOUND*r;
			unsigned my_count = 0;
			unsigned char buf[96];
			unsigned char my_set[12];
			for(unsigned s = s_hi; s < s_hi + S_LO_BOUND; ++s){
				for(unsigned char i = 0; i < rush_len; ++i){
					buf[i] = i;
				}
				for(unsigned i = 0; i < rush_len; ++i){
					// we need to compute the i'th output of the rng, which is
					// (AS[i] * s + BS[i])%2147483647, which we can compute using some silly bitshifts
					unsigned t_lo = AS[i] * s;
					unsigned t_hi = mul_hi(AS[i], s);
					unsigned x = (t_lo & 2147483647u) + (t_lo >> 31) + (t_hi << 1);
					x += BS[i];
					if(x >= 2147483647u){
						x -= 2147483647u;
					}
					unsigned j = (unsigned)( (double)x/2147483647.*(double)rush_len );
					unsigned char t = buf[i];
					buf[i] = buf[j];
					buf[j] = t;
				}
				int is_good = 1;
				for(unsigned char i = 0; i < subset_rules_len; ++i){
					for(unsigned char j = 0; j < 12; ++j){
						my_set[j] = 0;
					}
					unsigned char a = subset_rules[(unsigned short)14*(unsigned short)i];
					unsigned char b = subset_rules[(unsigned short)14*(unsigned short)i + (unsigned short)1];
					for(unsigned char j = a; j < b; ++j){
						unsigned char k = buf[j];
						my_set[k>>3] |= (unsigned char)1 << (k&(unsigned char)7);
					}
					for(unsigned char j = 0; j < 12; ++j){
						is_good &=
							!(~my_set[j] & subset_rules[(unsigned short)14*(unsigned short)i + (unsigned short)2 + (unsigned short)j]);
						// normally we would break, but that would make the gpu SAD
					}
				}
				for(unsigned short off = 0; off < sequence_rules_size;){
					unsigned char a = sequence_rules[off++];
					unsigned char b = sequence_rules[off++];
					for(unsigned char j = 0; j < b-a; ++j){
						is_good &= buf[a+j] == sequence_rules[off + (unsigned)j];
					}
					off += (unsigned)(b-a);
				}
				if(is_good && my_count < 16){
					out[16*r + my_count++] = s;
				}
			}
		}
	"#;
	let (p, d) = 'a: {
		for p in ocl::Platform::list() {
			for d in ocl::Device::list(p, Some(ocl::DeviceType::GPU))? {
				break 'a (p, d);
			}
		}
		return AOk(None);
	};
	// println!("DEBUG: starting GPU setup");
	let mut pro_que = ProQue::builder().platform(p).device(d).src(src).build().context("Compiling GPU code")?;
	println!("DEBUG: selected GPU \"{}\"", pro_que.device().name()?);
	let wg_size = 1 << 20;
	pro_que.set_dims(wg_size);
	AOk(Some(Gpu { pro_que, wg_size }))
}

pub fn find_matching_seeds_gpu(level_count: usize, result_count: usize, rules: &[Rule], gpu: &Gpu) -> AResult<impl Iterator<Item=i32>> {
	let out_buffer = gpu.pro_que.buffer_builder::<u32>().len(16*gpu.wg_size).build().context("Creating GPU result buffer")?;
	let mut subset_rules = Vec::new();
	let mut sequence_rules = Vec::new();
	for rule in rules {
		match rule {
			Rule::Subset(a, b, s) => {
				subset_rules.push((*a)as _);
				subset_rules.push((*b)as _);
				for b in 0..12 {
					let mut x = 0;
					for a in 0..8 {
						if s.contains(b*8 + a) {
							x |= 1 << a;
						}
					}
					subset_rules.push(x);
				}
			},
			Rule::Sequence(a, b, v) => {
				sequence_rules.push((*a)as _);
				sequence_rules.push((*b)as _);
				sequence_rules.extend(v.iter().copied());
			}
		}
	}
	subset_rules.push(0); // add dummy bytes due to an OpenCL requirement that buffers cannot have size 0
	sequence_rules.push(0);
	let subset_rules_buffer = gpu.pro_que.buffer_builder::<u8>()
		.len(subset_rules.len()).copy_host_slice(&subset_rules)
		.build().context("Creating GPU subset rule buffer")?;
	let sequence_rules_buffer = gpu.pro_que.buffer_builder::<u8>()
		.len(sequence_rules.len()).copy_host_slice(&sequence_rules)
		.build().context("Creating GPU sequence rule buffer")?;
	let kernel = gpu.pro_que.kernel_builder("gather_matching_seeds")
		.arg(&out_buffer).arg(level_count as u8)
		.arg((subset_rules.len()/14)as u8).arg(sequence_rules.len()as u16-1)
		.arg(&subset_rules_buffer).arg(&sequence_rules_buffer)
		.build().context("Preparing GPU call")?;
	// println!("DEBUG: all buffers are ready");
	unsafe { kernel.enq().context("Calling GPU code")?; }
	// println!("DEBUG: GPU code finished!");
	let mut out = vec![0; out_buffer.len()];
	// println!("DEBUG: result buffer allocated, doing read");
	out_buffer.read(&mut out).enq().context("Reading GPU result")?;
	// println!("DEBUG: read completed!");
	AOk(out.into_iter().filter_map(|x|match x { u32::MAX => None, x => Some(x as _) }).take(result_count))
}

