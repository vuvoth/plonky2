use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::fri::oracle::PolynomialBatch;
use plonky2::fri::proof::{CompressedFriProof, FriProof};
use plonky2::fri::structure::{FriOpeningBatch, FriOpenings};
use plonky2::hash::hash_types::RichField;
use plonky2::hash::merkle_tree::MerkleCap;
use plonky2::iop::challenger::Challenger;
use plonky2::plonk::config::{GenericConfig, Hasher};
use plonky2::plonk::proof::FriChallenges;
use rayon::prelude::*;

pub struct StarkProof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> {
    /// Merkle cap of LDEs of trace values.
    pub trace_cap: MerkleCap<F, C::Hasher>,
    /// Merkle cap of LDEs of trace values.
    pub quotient_polys_cap: MerkleCap<F, C::Hasher>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: StarkOpeningSet<F, D>,
    /// A batch FRI argument for all openings.
    pub opening_proof: FriProof<F, C::Hasher, D>,
}

pub struct StarkProofWithPublicInputs<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub proof: StarkProof<F, C, D>,
    // TODO: Maybe make it generic over a `S: Start` and replace with `[F; S::PUBLIC_INPUTS]`.
    pub public_inputs: Vec<F>,
}

pub struct CompressedStarkProof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    /// Merkle cap of LDEs of trace values.
    pub trace_cap: MerkleCap<F, C::Hasher>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: StarkOpeningSet<F, D>,
    /// A batch FRI argument for all openings.
    pub opening_proof: CompressedFriProof<F, C::Hasher, D>,
}

pub struct CompressedStarkProofWithPublicInputs<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub proof: CompressedStarkProof<F, C, D>,
    pub public_inputs: Vec<F>,
}

pub(crate) struct StarkProofChallenges<F: RichField + Extendable<D>, const D: usize> {
    // Random values used to combine PLONK constraints.
    pub stark_alphas: Vec<F>,

    // Point at which the PLONK polynomials are opened.
    pub stark_zeta: F::Extension,

    pub fri_challenges: FriChallenges<F, D>,
}

/// Purported values of each polynomial at the challenge point.
pub struct StarkOpeningSet<F: RichField + Extendable<D>, const D: usize> {
    pub local_values: Vec<F::Extension>,
    pub next_values: Vec<F::Extension>,
    pub permutation_zs: Vec<F::Extension>,
    pub quotient_polys: Vec<F::Extension>,
}

impl<F: RichField + Extendable<D>, const D: usize> StarkOpeningSet<F, D> {
    pub fn new<C: GenericConfig<D, F = F>>(
        zeta: F::Extension,
        g: F::Extension,
        trace_commitment: &PolynomialBatch<F, C, D>,
        quotient_commitment: &PolynomialBatch<F, C, D>,
    ) -> Self {
        let eval_commitment = |z: F::Extension, c: &PolynomialBatch<F, C, D>| {
            c.polynomials
                .par_iter()
                .map(|p| p.to_extension().eval(z))
                .collect::<Vec<_>>()
        };
        Self {
            local_values: eval_commitment(zeta, trace_commitment),
            next_values: eval_commitment(zeta * g, trace_commitment),
            permutation_zs: vec![/*TODO*/],
            quotient_polys: eval_commitment(zeta, quotient_commitment),
        }
    }

    // Note: Can't implement this directly on `Challenger` as it's in a different crate.
    pub fn observe<H: Hasher<F>>(&self, challenger: &mut Challenger<F, H>) {
        let StarkOpeningSet {
            local_values,
            next_values,
            permutation_zs,
            quotient_polys,
        } = self;
        for v in &[local_values, next_values, permutation_zs, quotient_polys] {
            challenger.observe_extension_elements(v);
        }
    }

    pub(crate) fn to_fri_openings(&self) -> FriOpenings<F, D> {
        let zeta_batch = FriOpeningBatch {
            values: [
                self.local_values.as_slice(),
                self.quotient_polys.as_slice(),
                self.permutation_zs.as_slice(),
            ]
            .concat(),
        };
        let zeta_right_batch = FriOpeningBatch {
            values: self.next_values.to_vec(),
        };
        FriOpenings {
            batches: vec![zeta_batch, zeta_right_batch],
        }
    }
}
