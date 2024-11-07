use elliptic_curve::{group::GroupEncoding, sec1::ToEncodedPoint, Field, PrimeField};
use k256::{
    schnorr::CryptoRngCore,
    sha2::{Digest, Sha256},
    ProjectivePoint, Scalar,
};
use serde::{Deserialize, Serialize};

/// Non-interactive Schnorr ZK DLOG Proof scheme with a Fiat-Shamir transformation.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DLogProof {
    #[serde(with = "projective_serializer")]
    t: ProjectivePoint,
    s: Scalar,
}

impl DLogProof {
    /// Creates a Schnorr ZK DLOG proof.
    ///
    /// `sid` is session ID and `pid` is participant ID.
    ///
    /// Given a private key `x` and its corresponding public key `y` = x*G, proves
    /// knowledge of `x` without revealing it.
    ///
    /// Returns `DLogProof` containing the commitment `t` and response `s`.
    pub fn prove(
        rng: &mut impl CryptoRngCore,
        sid: &str,
        pid: u32,
        x: Scalar,
        y: ProjectivePoint,
    ) -> Self {
        let r = Scalar::random(rng);
        let t = ProjectivePoint::GENERATOR * r;
        let c = Self::hash_points(sid, pid, &[ProjectivePoint::GENERATOR, y, t]);
        let s = r + c * x;

        DLogProof { t, s }
    }

    /// Verifies a Schorr ZK DLOG Proof using the discrete logarithm `x` of y = x*G
    /// without revealing x.
    /// 
    /// `sid` is session ID and `pid` is participant ID.
    /// 
    /// Returns `true` if the proof is valid, `false` otherwise.
    pub fn verify(&self, sid: &str, pid: u32, y: ProjectivePoint) -> bool {
        let c = Self::hash_points(sid, pid, &[ProjectivePoint::GENERATOR, y, self.t]);
        let lhs = ProjectivePoint::GENERATOR * self.s;
        let rhs = self.t + (y * c);

        lhs == rhs
    }

    fn hash_points(sid: &str, pid: u32, points: &[ProjectivePoint]) -> Scalar {
        let mut hasher = Sha256::new();
        hasher.update(sid);
        hasher.update(pid.to_be_bytes());
        for point in points {
            hasher.update(point.to_bytes());
        }
        let digest = hasher.finalize();

        Scalar::from_repr(digest).expect("sha256 should be a valid scalar")
    }
}

// We use SEC1 encoding format without compression for serialization/deserialization.
mod projective_serializer {
    use elliptic_curve::{
        sec1::{EncodedPoint, FromEncodedPoint},
        AffinePoint,
    };
    use k256::Secp256k1;
    use serde::{Deserializer, Serializer};

    use super::*;

    pub fn serialize<S>(point: &ProjectivePoint, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded_point = point.to_affine().to_encoded_point(false);

        encoded_point.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ProjectivePoint, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded_point = EncodedPoint::<Secp256k1>::deserialize(deserializer)?;

        AffinePoint::<Secp256k1>::from_encoded_point(&encoded_point)
            .into_option()
            .ok_or_else(|| serde::de::Error::custom("Invalid point encoding"))
            .map(|point| point.into())
    }
}

#[cfg(test)]
mod tests {
    use k256::elliptic_curve::rand_core;

    use super::*;

    #[test]
    fn valid_proof() {
        let mut rng = rand_core::OsRng;
        let sid = "sid";
        let pid = 1;
        let x = Scalar::random(&mut rng);
        let y = ProjectivePoint::GENERATOR * x;

        let proof = DLogProof::prove(&mut rng, sid, pid, x, y);

        assert!(proof.verify(sid, pid, y))
    }

    #[test]
    fn invalid_proof_with_different_sessions() {
        let mut rng = rand_core::OsRng;
        let sid = "sid1";
        let pid = 1;
        let x = Scalar::random(&mut rng);
        let y = ProjectivePoint::GENERATOR * x;

        let proof = DLogProof::prove(&mut rng, sid, pid, x, y);

        assert!(!proof.verify("sid2", pid, y))
    }

    #[test]
    fn invalid_proof_with_different_participants() {
        let mut rng = rand_core::OsRng;
        let sid = "sid1";
        let pid = 1;
        let x = Scalar::random(&mut rng);
        let y = ProjectivePoint::GENERATOR * x;

        let proof = DLogProof::prove(&mut rng, sid, pid, x, y);

        assert!(!proof.verify(sid, 2, y))
    }

    #[test]
    fn serialization_roundtrip() {
        let mut rng = rand_core::OsRng;
        let sid = "sid";
        let pid = 1;
        let x = Scalar::random(&mut rng);
        let y = ProjectivePoint::GENERATOR * x;

        let original_proof = DLogProof::prove(&mut rng, sid, pid, x, y);

        let json_proof =
            serde_json::to_string(&original_proof).expect("serialization should succeed");

        let decoded_proof =
            serde_json::from_str(&json_proof).expect("deserialization should succeed");

        assert_eq!(original_proof, decoded_proof);
        assert!(decoded_proof.verify(sid, pid, y));
    }
}
