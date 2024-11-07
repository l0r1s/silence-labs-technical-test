use std::time::Instant;

use dlog_proof::DLogProof;
use elliptic_curve::{rand_core, Field};
use k256::{ProjectivePoint, Scalar};

pub fn main() {
    let mut rng = rand_core::OsRng;

    let sid = "sid";
    let pid = 1;

    let x = Scalar::random(&mut rng);
    let y = ProjectivePoint::GENERATOR * x;

    println!("Randomly chosen x:");
    println!("{:?}", x);

    println!("");

    let start_proof = Instant::now();
    let dlog_proof = DLogProof::prove(&mut rng, sid, pid, x, y);
    println!(
        "Proof computation time: {} ms",
        start_proof.elapsed().as_millis()
    );

    println!("");

    println!(
        "Proof: \n{}",
        serde_json::to_string_pretty(&dlog_proof).expect("Serialization failed")
    );

    println!("");

    let start_verify = Instant::now();
    let result = dlog_proof.verify(sid, pid, y);
    println!(
        "Verify computation time: {} ms",
        start_verify.elapsed().as_millis()
    );

    if result {
        println!("DLOG proof is correct")
    } else {
        println!("DLOG proof is not correct")
    }
}
