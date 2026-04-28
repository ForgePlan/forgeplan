---
kind: component
name: Database
parent_container: data-layer
owner: platform-team
---

# Database

## Purpose

Primary relational store for user accounts, audit events, and durable
domain entities. Single-writer Postgres cluster with hot-standby read
replicas. Source of truth for everything not in object storage.

## Responsibilities

- Persist user accounts and authentication events
- Enforce referential integrity on domain tables
- Stream WAL to standby replicas for failover
- Take nightly logical backups to object storage

## Consumers

- Auth Service (read users, write auth_events)
- API Gateway (read sessions cache fallback)
- Reporting Worker (read-replica only)
