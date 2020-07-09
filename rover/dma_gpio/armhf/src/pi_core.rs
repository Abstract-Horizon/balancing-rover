//! Only accessable when 'bind_process' feature is on. Module for binding process to last core.

use hwloc::{Topology, TopologyObject, ObjectType, CPUBIND_PROCESS};


pub fn bind_process_to_last() {

    let mut topo = Topology::new();

    let mut cpuset = last_core(&mut topo).cpuset().unwrap();
    cpuset.singlify();

    match topo.set_cpubind(cpuset, CPUBIND_PROCESS) {
        Ok(_) => trace!("Correctly bound process to last core"),
        Err(e) => error!("{:?}", e),
    }
}

fn last_core(topo: &mut Topology) -> &TopologyObject {
    let core_depth = topo.depth_or_below_for_type(&ObjectType::Core).unwrap();
    let all_cores = topo.objects_at_depth(core_depth);
    all_cores.last().unwrap()
}
