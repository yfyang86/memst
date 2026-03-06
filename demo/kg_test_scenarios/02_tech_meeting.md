# Tech Meeting: Project Phoenix Planning

## Attendees
- Sarah Chen (Engineering Manager)
- Alex Rodriguez (Tech Lead)
- Jordan Kim (Product Manager)
- Morgan Taylor (DevOps Lead)

---

Sarah: "Alright team, welcome to the Project Phoenix kickoff. We're here to plan the migration from our legacy monolith to microservices. Jordan, can you give us the timeline overview?"

Jordan: "Sure. We're targeting Q3 for the initial API gateway launch, with full migration completed by end of Q4. The critical path is the payment service - that needs to be live by July 15th for the holiday shopping prep."

Alex: "July 15th is tight. The payment service has 47 external integrations. We need at least 6 weeks for testing alone."

Morgan: "I can help with that. If we use the new Kubernetes cluster in us-west-2, we can spin up isolated test environments on demand. That should cut testing time by 30%."

Sarah: "Good. What about the database migration? That's our biggest risk."

Alex: "We're planning to use the strangler fig pattern. We'll keep PostgreSQL as the source of truth, gradually move read traffic to the new service databases. MongoDB for product catalog, Redis for sessions, Cassandra for analytics."

Jordan: "Product catalog migration is my concern. We have 2.3 million SKUs. What's the ETA?"

Alex: "3 weeks for the initial data migration, then 2 weeks of parallel running. Morgan, can your team handle the infrastructure?"

Morgan: "Yes, but I need to order 8 more application servers. Current capacity won't handle both systems running in parallel."

Sarah: "Approved. Get the PO out today. Any blockers?"

Alex: "The Stripe API v2 migration. It's a breaking change that affects the payment service. Stripe is deprecating v1 on August 1st."

Jordan: "That's non-negotiable then. We have to migrate to v2 before go-live."

Sarah: "Agreed. Let's add that as a dependency. Morgan, flag any infrastructure costs by EOD. Alex, I need a detailed technical design doc by Friday. Jordan, update the roadmap and communicate the July 15th deadline to stakeholders."

All: "Got it."

Sarah: "Great. Let's make this happen. Phoenix rises from the ashes - let's make sure our system does too. Meeting adjourned."

---

## Action Items
1. Morgan: Submit PO for 8 application servers - Due Today
2. Alex: Technical design document - Due Friday
3. Jordan: Update roadmap and communicate deadline - Due Tomorrow
4. Alex: Stripe API v2 migration plan - Due Next Week

## Key Dates
- July 15: Payment service go-live
- Q3: API gateway launch
- Q4: Full migration complete
- August 1: Stripe v1 deprecation (external deadline)

---

## Expected KG Elements

### Entities (People)
- Sarah Chen (Engineering Manager)
- Alex Rodriguez (Tech Lead)
- Jordan Kim (Product Manager)
- Morgan Taylor (DevOps Lead)

### Entities (Organizations)
- Stripe (External vendor)

### Entities (Projects)
- Project Phoenix (Microservices migration)

### Entities (Technologies)
- PostgreSQL (Primary database)
- MongoDB (Product catalog)
- Redis (Sessions)
- Cassandra (Analytics)
- Kubernetes (Orchestration)
- API Gateway
- Stripe API v2

### Entities (Locations)
- us-west-2 (AWS region)

### Relationships
- Sarah manages the team
- Alex leads technical decisions
- Jordan manages product timeline
- Morgan handles infrastructure
- Project Phoenix uses Kubernetes
- Payment service depends on Stripe API

### Events
- Project Phoenix kickoff meeting
- July 15 payment service deadline
- August 1 Stripe v1 deprecation

### Decisions
- Use strangler fig pattern for migration
- Purchase 8 additional servers
- Target Q3 for API gateway
