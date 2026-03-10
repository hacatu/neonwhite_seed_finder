use itertools::Itertools;
use anyhow::{Result as AResult, Ok as AOk, Context};
use ocl::{ProQue};
use crate::backend::{AS, BS, Rule};

pub struct Gpu {
	pro_que: ProQue,
	x_los_buffer: ocl::Buffer<u32>,
	wg_size: usize
}

pub fn try_setup_gpu() -> AResult<Option<Gpu>> {
	/*
	The basic idea is that the nth output of the prng with seed s has form A*s + B, for some constants A and B, so we can decompose
	every s as s_hi + s_lo, and then (A*s_hi) + (A*s_lo + B).  We will precompute (A*s_lo + B) for every possible value of s_lo, and
	the threads will compute (A*s_hi) for some s_hi computed from their thread id.  In particular, if the max global work group size
	is say 2^20, we will have s_hi be 20 bits long and s_lo be 11 bits long, etc.
	*/
	let src = r#"
		__kernel void gather_matching_seeds(
			__global unsigned *out,
			__global const unsigned *x_los,
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
			unsigned x_his[96] = {1121899819, 630111683, 1501065279, 458365203, 969558243, 1876681249, 962194431, 1077359051, 265679591, 791886952, 1582116761, 1676571504, 1476289907, 1117239683, 1503178135, 1341148412, 902714229, 1331438416, 58133212, 831516153, 285337308, 526856546, 362935496, 750214563, 210465667, 1381224997, 1846331200, 1330597961, 593162892, 1729496551, 792803163, 565661843, 863554642, 53838754, 749855384, 93067682, 1778866589, 1463507567, 367760674, 1219347826, 1648614489, 596622148, 1228675679, 243017841, 1132230640, 1891159862, 730619752, 33642253, 209795643, 283831563, 249493290, 967871855, 1560699908, 437500212, 429989927, 595043273, 267176187, 750850716, 247899536, 1735816893, 30350049, 1779080117, 484196159, 683666687, 2146567436, 1016454918, 813016862, 1422451153, 367384299, 1410110453, 1709765470, 1586690309, 963677742, 986269033, 1330385311, 1836198807, 1445664514, 119917655, 1765467570, 466789452, 650605245, 1812688947, 1120802318, 309331329, 1480003261, 1972414955, 1152445582, 426054430, 1771332474, 154812111, 1973375142, 1028015873, 1215608031, 779427428, 1188997777, 2017018019};
			for(unsigned char i = 0; i < rush_len; ++i){
				x_his[i] = (unsigned)((unsigned long)x_his[i]*(unsigned long)s_hi%(unsigned long)2147483647);
			}
			unsigned my_count = 0;
			unsigned char buf[96];
			unsigned char my_set[12];
			for(unsigned s_lo = 0; s_lo < S_LO_BOUND; ++s_lo){
				for(unsigned char i = 0; i < rush_len; ++i){
					buf[i] = i;
				}
				for(unsigned i = 0; i < rush_len; ++i){
					unsigned x = x_his[i] + x_los[s_lo*(unsigned)96 + i];
					if(x >= (unsigned)2147483647){
						x -= (unsigned)2147483647;
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
						if(~my_set[j] & subset_rules[(unsigned short)14*(unsigned short)i + (unsigned short)2 + (unsigned short)j]){
							is_good = 0; // normally we would break, but that would make the gpu SAD
						}
					}
				}
				for(unsigned short off = 0; off < sequence_rules_size;){
					unsigned char a = sequence_rules[off++];
					unsigned char b = sequence_rules[off++];
					for(unsigned char j = 0; j < b-a; ++j){
						if(buf[a+j] != sequence_rules[off + (unsigned)j]){
							is_good = 0;
						}
					}
					off += (unsigned)(b-a);
				}
				if(is_good && my_count < 16){
					out[16*r + my_count++] = s_hi + s_lo;
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
	println!("DEBUG: ocl selected OpenCL device {}", pro_que.device().name()?);
	let wg_size = 1 << 20;
	pro_que.set_dims(wg_size);
	let s_lo_bound = (1 << 31)/wg_size as u32;
	// println!("DEBUG: s_lo is < {s_lo_bound}");
	let x_los = (0..s_lo_bound).flat_map(|s|AS.iter().zip(&BS).map(move|(&a, &b)|((a as u64*s as u64+b as u64)%i32::MAX as u64)as _)).collect_vec();
	let x_los_buffer = pro_que.buffer_builder::<u32>().len(x_los.len()).copy_host_slice(&x_los).build().context("Creating GPU lookup table for low bits of rng outputs")?;
	AOk(Some(Gpu { pro_que, x_los_buffer, wg_size }))
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
		.arg(&out_buffer).arg(&gpu.x_los_buffer).arg(level_count as u8)
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

