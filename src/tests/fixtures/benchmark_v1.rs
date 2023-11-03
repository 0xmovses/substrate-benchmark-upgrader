// static fixture for testing

benchmarks_instance_pallet! {
	// This tests when proposal is created and queued as "proposed"
	propose_proposed {
		let b in 1 .. MAX_BYTES;
		let m in 2 .. T::MaxFellows::get();
		let p in 1 .. T::MaxProposals::get();

		let bytes_in_storage = b + size_of::<Cid>() as u32 + 32;

		// Construct `members`.
		let fellows = (0 .. m).map(fellow::<T, I>).collect::<Vec<_>>();
		let proposer = fellows[0].clone();

		Alliance::<T, I>::init_members(
			SystemOrigin::Root.into(),
			fellows,
			vec![],
		)?;

		let threshold = m;
		// Add previous proposals.
		for i in 0 .. p - 1 {
			// Proposals should be different so that different proposal hashes are generated
			let proposal: T::Proposal = AllianceCall::<T, I>::set_rule {
				rule: rule(vec![i as u8; b as usize])
			}.into();
			Alliance::<T, I>::propose(
				SystemOrigin::Signed(proposer.clone()).into(),
				threshold,
				Box::new(proposal),
				bytes_in_storage,
			)?;
		}

		let proposal: T::Proposal = AllianceCall::<T, I>::set_rule { rule: rule(vec![p as u8; b as usize]) }.into();

	}: propose(SystemOrigin::Signed(proposer.clone()), threshold, Box::new(proposal.clone()), bytes_in_storage)
	verify {
		// New proposal is recorded
		let proposal_hash = T::Hashing::hash_of(&proposal);
		assert_eq!(T::ProposalProvider::proposal_of(proposal_hash), Some(proposal));
	}
}