# Teleport: one time programs on Twitter

Teleport creates "post once" links for your twitter account. Each link is destroyed after one use.

When you create an account link, you can also define a safeguard policy that will be enforced by an LLM whenever somebody tries to redeem it. For example, to prevent spamming and inappropriate content, you can define a policy of `“only allow posts about cute cats that have good positive pro-social vibes."` Because the safeguard is implemented as a GPT-4o API call, you can choose to gate the posting by conditioning on external events such as
`"only allow posting if Germany wins Spain in Eurocup 2024 on https://www.uefa.com/euro2024/fixtures-results"`.

Each account link is represented as an NFT that's minted to your on-chain address (the link sharing is done by creating a unique identifier to represent the ownership of that NFT). Once minted, the NFT can be auctioned off, put into an AMM pool, used as collateral, traded, or simply gifted.

Those of you with web2 instincts will readily spot the potential to "sell ads" on your twitter, but we encourage more innovative and/or wholesome uses: let your friends roast you on your own X account if you lose a bet, tweet spicy takes without having to prepend "OH", or simply "lend" your established social media voice to those without a following who need the boost.

## Key Features

- *Programmable Access:* you can easily define AI-driven "social contracts" in the form of LLM safeguards on top of your one-time posting account links. Because those links are represented as NFTs, they can be easily composed with permissionless on-chain infra: auctions, exchanges.
- Auditable Security: users can verify the system's integrity through certificate transparency and trusted-execution-environments (TEE) remote attestation. One post is one post, and there is no skipping of the LLM safeguard.

The backend repo is open source at: <https://github.com/Account-Link/teleport-gramine-rs>

The measurement for the backend enclave is:  `167b9f8b19388d7e52cc9a249ba5a5aac964324cbd7298c66c866278df4889c5`

The smart contract address is: [0xe1c4c77c45081dab2eba1d8af9eb468ea6c5cdd8](https://basescan.org/address/0xe1c4c77c45081dab2eba1d8af9eb468ea6c5cdd8)

## Quickstart

To set up the development environment for Teleport, follow these steps:

1. **Prerequisites**
   - Docker
   - Make sure you have access to SGX-enabled x86_64 hardware running Linux

2. **Clone the Repository**

   ```bash
   git clone https://github.com/Account-Link/teleport-gramine-rs.git
   cd teleport-gramine-rs
   ```

3. **Environment Configuration**
   - Copy `private.env.example` to `private.env`
   - Fill in the necessary environment variables in `private.env`

4. **Build and Run**
   - To build the Docker image:

     ```bash
     docker compose build
     ```

   - To start the server:

     ```bash
     docker compose run --rm teleport "gramine-sgx exex"
     ```

5. **Development Workflow**
   - Make changes to the code on your host machine
   - Repeat step 4 to restart the server with the updated code

For more detailed information about the project structure, available commands, and contribution guidelines, please refer to our [Development Guide](./docs/DEVELOPMENT.md).

## How does Teleport work?

The core technology backing teleport is trusted-execution-enclaves (TEEs). Our idea is based on a series of work on [secure account delegation](https://eprint.iacr.org/2018/160), [one-time programs](https://iacr.org/archive/crypto2008/51570039/51570039.pdf), and [complete knowledge](https://eprint.iacr.org/2023/044). Read about our approach [here](https://drive.google.com/file/d/1qIX22m7mqBK9TcElpBCAjYuPjSmKRap8/view).

## What does the TEE provide in Teleport?

The TEE is accomplishing two things for us:

1. Our service cannot exceed the stated use. One post means one post. And no skipping the content filter.

2. You don’t have to take our word about 1, you can check it for yourself.

For more details on the security model and how to validate the remote attestation process, see [AUDITING.md](./AUDITING.md)

## Is Teleport making some deeper point about the nature of property?

Property rights are limited by enforceability. As we move into a new digital age with smart contracts and TEEs, enforcement becomes much more powerful, making new kinds of property rights become viable.

Actually this is part of a broad trend where value capture moves upwards in the supply chain, towards user-facing apps and end users themselves.

- Past: “action” (physical property, labor)
- Present: “intention” (organizations, intellectual property)
- Future: “attention” (liquefied virtual property, sensitive information)

**We want to solve the scalable value exchange problem as property moves to the information level.**

Correlation and mediation of property is happening at the information level inside Google’s server room with you and your friend’s data, instead of happening at the physical level of negotiation and barter. This trend expands the boundary property and makes the value exchange more efficient, but we see one major problem right now:

*The technology that can scale value exchange for information properties is decoupled from the majority of information properties.* Crypto excels at composability and exchange, and web2 has a lot of valuable property, but due to poor interoperability (read/write) between them:

- web3 people are forced to create valuable assets on-chain natively to bootstrap use cases (solution finding problem) instead of trying to solve huge pain points that already exist for the massive Internet users today
- web2 people are forced to create non-functional markets to exchange value. If today I wanted to exchange or even delegate private digital resources such as my accounts, there is simply no platform to do that with low friction.

We see TEE offering the value exchange highway for the information age because it allows shared computation over private state. It brings web2 distribution channel to web3 and web3 programmability and composability to web2.

We take an approach to make tools that are auditable for safety (avoids excessive use) and only looks for synergetic wholesome use cases. We strive to create a secure, ethical, and innovative ecosystem that respects both user privacy and platform integrity.

## Cool use of LLM. But what does this really have to do with AI?

We consider Teleport to be a first step towards making multi-agent AI interactions real.

Instead of trying to make AIs pay each other using Bitcoin, now you can directly share an AI that has access to your social capital (which is programmable money in the higher dimensional value space, the LLM safeguard you define is quite literally a "social contract"). Sharing a link to make a bet with your friend to post something is a very tangible and lightweight experiment for cooperative AI with credible commitments over private digital resources. We want to start with “small, open-source model friendly use cases.”

We are optimistic about an autonomous future where everyone can share intelligent one-time-programs (or, self-enforcing commitments with privacy and integrity guarantees) with others securely.

In short, we are *using web2 accounts as the substrate for autonomous agents to run on*.

## Now what?

- Teleport is our project submission to the TEE/acc movement. We thank [Flashbots](https://www.flashbots.net/) and [IC3](https://www.initc3.org/) for their extensive support and collaboration.
- We are staunchly supportive of free open source code. TEEs are for everyone to use. The right to fractionalize and redelegate your web2 accounts belongs to everyone.
- Our plan is to earn leadership through accelerated development and public outreach, then use it to stimulate creative and wholesome uses so the technology.

## Disclaimers

This is not production code. It has not been audited. It is a provocative stunt! Use it at your own risk.

We are demonstrating a security-motivated *design*, which is all about using TEE to minimize trust in the server.
Eventually it could even be run as a completely decentralized p2p network.
But for now we're simply hosting this ourselves.
Although instructions to auditors can be found in AUDITING.md, we have not yet commissioned any professional audit.

This is the last time we write anything in Gramine-SGX. Onward to TDX!
