/*
let physinfo = xen.physinfo();
println!("{physinfo:#?}");

if let Ok(physinfo) = physinfo {
    let mut cpuinfos = vec![MaybeUninit::uninit(); physinfo.nr_cpus as usize];
    let infos = xen.get_cpuinfo(&mut cpuinfos);

    println!("{infos:#?}");
}
*/
