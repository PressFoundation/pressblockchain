# SYNC — Article Submission + Live Vote Bar

This plugin integrates with Press Articles API (press-articles service):
- POST /v1/articles/submit
- POST /v1/articles/vote
- GET  /v1/articles/:id

Implementation notes:
- Article submission page requires PRESS payment + AI content moderation pass.
- Post remains in WP "pending" state until approved.
- Vote bar polls GET /v1/articles/:id for live counts.
