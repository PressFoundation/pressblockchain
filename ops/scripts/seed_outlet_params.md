# Seed Outlet Params (RR26)

These parameters are read by OutletRegistry and OutletTokenFactory.

Keys (keccak):
- outlet_create_fee
- outlet_bond_min
- outlet_token_deploy_fee
- outlet_token_list_fee (future)
- outlet_role_bond_min (future)

Recommended defaults (tune later):
- outlet_create_fee: 250 * 1e18 (250 PRESS)
- outlet_bond_min:  5000 * 1e18 (5000 PRESS)
- outlet_token_deploy_fee: 1000 * 1e18 (1000 PRESS)

RR26 adds these as **suggested presets**; actual setting should be executed via governance (param change + council execution).
