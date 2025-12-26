RR18 requires deploy.json include:
- councilExecutor
- pressParameters

These are deployed alongside ProposalCenter and CouncilRegistry and are used for:
- council multisig-style execution of passing PARAM_CHANGE proposals
- canonical parameter store (bytes32 -> uint256)
