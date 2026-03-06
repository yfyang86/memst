# Software Architecture: E-Commerce Platform Design

## System Context
Designing the next-generation e-commerce platform for "ShopMart", handling 10M+ daily active users.

## Authors
- Lead Architect: Dr. Samantha Lee
- Senior Engineer: David O'Brien
- Date: February 2025

---

## Executive Summary

ShopMart's current monolithic architecture cannot scale to meet projected 2026 growth. We're proposing a distributed microservices architecture using event-driven patterns with Apache Kafka as the central nervous system.

---

## Core Services

### 1. API Gateway
**Technology:** Kong Gateway 3.5
**Responsibilities:**
- Rate limiting (10,000 req/min per API key)
- Authentication via JWT tokens
- Request routing to 12 downstream services
- SSL termination

### 2. User Service
**Technology:** Node.js + Express + PostgreSQL
**Domain:** User management, authentication, profiles
**Scale:** 50M user records
**Caching:** Redis cluster (3 master, 6 replicas)

### 3. Product Catalog Service
**Technology:** Java + Spring Boot + MongoDB
**Domain:** Product information, categories, search
**Special Features:**
- Full-text search via Elasticsearch
- Image CDN integration (CloudFront)
- Real-time inventory from Inventory Service

### 4. Shopping Cart Service
**Technology:** Go + Redis
**Domain:** Cart management, persistence
**Pattern:** Event sourcing with CQRS
- Command side: Handle add/remove/update
- Query side: Read-optimized cart views

### 5. Order Service
**Technology:** Java + Spring Boot + PostgreSQL
**Domain:** Order processing, lifecycle management
**Integration:**
- Payment Service (Stripe, PayPal)
- Inventory Service (reservation pattern)
- Shipping Service (UPS, FedEx APIs)

### 6. Payment Service
**Technology:** Python + FastAPI
**Domain:** Payment processing, PCI compliance
**Security:** Tokenization, encrypted vault
**Providers:** Stripe (primary), PayPal (secondary)

### 7. Notification Service
**Technology:** Node.js + AWS SNS/SQS
**Domain:** Email, SMS, push notifications
**Channels:**
- SendGrid (email)
- Twilio (SMS)
- Firebase Cloud Messaging (push)

---

## Data Architecture

### Relational Databases (PostgreSQL)
- User data (sharded by user_id)
- Order transactions (sharded by date)
- Payment records (encrypted, PCI scope)

### NoSQL Databases
- **MongoDB:** Product catalog, unstructured content
- **Redis:** Sessions, caching, rate limiting
- **Elasticsearch:** Search indexes, analytics
- **Cassandra:** Time-series data, clickstream

### Event Store
- **Apache Kafka:** Event bus, 7-day retention
- Topics:
  - `user.events` (registrations, logins)
  - `order.events` (created, paid, shipped)
  - `inventory.events` (stock changes)
  - `notification.events` (triggered notifications)

---

## Communication Patterns

### Synchronous (REST/gRPC)
- User Service ↔ Authentication
- Order Service ↔ Payment Service
- Product Service ↔ Elasticsearch

### Asynchronous (Events/Kafka)
- Order placed → Inventory reservation
- Payment confirmed → Order status update
- User registered → Welcome email triggered
- Inventory low → Reorder alert

---

## Scalability Strategy

### Horizontal Pod Autoscaling
- Min replicas: 3
- Max replicas: 50
- Target CPU: 70%
- Target memory: 80%

### Database Scaling
- Read replicas: 5 per shard
- Write sharding: 16 shards
- Connection pooling: HikariCP (Java), PgBouncer (PostgreSQL)

### CDN Strategy
- Static assets: CloudFront
- Product images: S3 + CloudFront
- API responses: 5-minute edge caching

---

## Security Architecture

### Authentication Flow
1. Client → API Gateway (JWT in header)
2. Gateway validates with Auth Service
3. Gateway adds `X-User-ID` header
4. Services use header for authorization

### Data Protection
- TLS 1.3 for all connections
- AES-256 encryption at rest
- Field-level encryption for PII
- Vault for secrets management

---

## Expected KG Elements

### Entities (People)
- Dr. Samantha Lee (Lead Architect)
- David O'Brien (Senior Engineer)

### Entities (Organizations)
- ShopMart (Company)
- Stripe (Payment provider)
- PayPal (Payment provider)
- UPS (Shipping)
- FedEx (Shipping)

### Entities (Systems/Services)
- API Gateway (Kong)
- User Service
- Product Catalog Service
- Shopping Cart Service
- Order Service
- Payment Service
- Notification Service
- Auth Service
- Inventory Service
- Shipping Service

### Entities (Technologies)
- Kong Gateway 3.5
- Node.js + Express
- Java + Spring Boot
- Go
- Python + FastAPI
- PostgreSQL
- MongoDB
- Redis
- Elasticsearch
- Apache Kafka
- Cassandra
- AWS SNS/SQS
- Kubernetes

### Entities (External Services)
- CloudFront (CDN)
- S3 (Storage)
- SendGrid (Email)
- Twilio (SMS)
- Firebase Cloud Messaging

### Relationships
- API Gateway routes to 12 services
- Order Service integrates with Payment Service
- Payment Service uses Stripe (primary)
- Product Catalog uses MongoDB
- User Service uses PostgreSQL
- Notification Service publishes to SendGrid
- All services produce events to Kafka

### Patterns
- Microservices architecture
- Event-driven architecture
- CQRS (Command Query Responsibility Segregation)
- Event sourcing
- API Gateway pattern
- Database per service
- Circuit breaker
- Rate limiting
