---
kind: component
name: Auth Service
parent_container: api-gateway
owner: platform-team
---

# Auth Service

## Purpose

The Auth Service authenticates incoming HTTP requests against the user
identity store and issues short-lived JWT access tokens plus longer-lived
refresh tokens. It is the only component in the system permitted to read
the password hash column on the `users` table.

## Responsibilities

- Validate username/password credentials against the identity store
- Issue, rotate, and revoke JWT access + refresh tokens
- Enforce per-account rate limits on failed login attempts
- Publish `auth.login.succeeded` and `auth.login.failed` events
- Expose JWKS endpoint for downstream services to verify tokens

## Consumers

- API Gateway (validates every inbound request)
- Web Frontend (login, logout, token refresh flows)
- Mobile Apps (login, biometric re-auth, token refresh)
- Admin Console (impersonation, force-logout)

## Dependencies

- Database (read-only on `users`, write on `auth_events`)
- Redis (rate-limit counters, session blocklist)

## Notes

The Auth Service is the security perimeter — it must never be bypassed.
All other components trust the JWT signature; only Auth holds the private
signing key.
