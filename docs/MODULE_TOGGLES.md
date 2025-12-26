# Module toggles (release gating)

## Principle
Press Blockchain must remain operational if any single module is disabled.

**Core services (must remain enabled):**
- RPC / JSON-RPC
- validator / sequencer
- base contracts required for chain boot

Everything else is a module.

## How toggles work
`config/modules.json` is the authoritative toggle map.
Installer reads this file and generates:
- docker compose profiles / service enablement
- frontend routes (feature flags)
- API route guards

## Safe-disable behavior
When a module is disabled:
- APIs return `403 MODULE_DISABLED` with a human message
- UI hides module menus and routes
- Chain remains functional

## Common modules
- marketplace
- outlet_wizard
- bots
- status_page
- burn_flywheel
- source_role
- earnings_vault
- syndication_engine
- dispute_bonds
- ai_verification_api

## Release strategy
Everything installs ON by default.
You can disable any module at install time to “hold back” features for future marketing.
