## Threat Model and Security Goals

An informal statement of our security goal is that users must intentionally authorize the creating of each limited use token.
Slightly more formally,
> If a one-time use token for X account `@user` is created with filter policy `P`, then the owner of `@user` saw the Authorization Window with a description of `P` and clicked Approve.
For example, if the user clicks Approve 5 times, no more than 5 posts should possible.

Teleport is designed to guarantee this through the use of a TEE server, even though we assume users just have ordinary browsers and no special plugins or client app.
In particular, although the backend `tee.teleport.best` is served from a TEE, the frontend at `teleport.best` is not in a TEE, and therefore we have to consider the frontend may be malicious. We must defend against attacks where a malicious frontend misleads the user into authorizing the creation of new tokens without realizing it.

In a nutshell, our approach has the TEE require the user to interact with an unskippable "Approval Window" before creating each limited-use token.

The following additional assumptions complete our threat model:
- We assume the user inspects the domain name `tee.teleport.best` before authorizing the application.
- We rely on the finality of the Base chain, as reported from the alchemy.com RPC service.
- Any certificate issued for the domain `tee.teleport.best` is assumed to appear in the Certificate Transparency (CT) logs, in particular `crt.sh`
- We are limited to auditability, rather than prevention. Any violation of these guarantees would require us to produce evidence causing the audit process described below to fail.

In the future we would like to relax some of these assumptions. We could ✨encumber✨ our Twitter developer account, such that we can show `tee.teleport.best` is the only domain name associated with our brand account. We could also host the entire front-end within a TEE, which could reduce the number of clicks while still ensuring users saw the correct message.

Finally, we only focus on what the user sees before creating a limited-use token, but we leave handling these tokens out of scope. For now these are shared with the untrusted front-end. Hosting the entire frontend within a TEE would improve this.

## How the software works

### Trustless domain name

Our first design goal is to ensure the domain `tee.teleport.best` is only served from a TEE running the right program.

First, the TEE backend generates a private key on startup, storing it in a sealed file so only the same backend that created it can access it.
A Certificate Signing Request (CSR) and a remote attestation quote are output to the host.
The host is responsible for using Let's Encrypt to validate their ownership of the domain name and produce a signed certificate endorsing that key.

Because every certificate issued for the domain appears in CT logs, auditors will be able to match every certificate to the quote that shows it was generated from the TEE.

### Authorization Window

The authorization flow is as follows:
  a user is shown a Twitter authorization window that redirects to `tee.teleport.best. Since this is configured by the frontend 
  - tee.teleport.best 

### One time redemption
Our TEE system uses the Base blockchain to ensure each token is truly "one use only", i.e. to prevent double spends.
Behind the scenes, the backend mints an NFT on Base chain for each one-time use code, and only posts to Twitter once the NFT is redeemed on-chain.

## Instructions to Auditors

### Specifying a release

The backend is tagged: `v0.1`

The MRENCLAVE is `6d63e420fbd32988baa7d1fceb467960faa48c7f53ff1d56d0384d0ceb910878`

### Building

To build and display the resulting enclave measurement, run
```bash
docker build -t teleport .
docker run --rm -it teleport
```

This displays the `mrenclave`, a hash of the entire enclave program including all the trusted data files and system libraries bundled with it.

The goal of the build process is to be reproducible, such that auditors in the future can rebuild the exact same enclave.
The base of the Dockerfile is `gramineproject/gramine:1.7-jammy`, a base image provided by Gramine. This serves as an anchor point.

The biggest limitation right now is having to install `build-essential`, which introduces system libraries that are subject to change as the package maintainers apply upgrades. As a workaround, we therefore also backing up the build image on Dockerhub, so that auditors can at least retrieve the image and interact with it this way. We should improve this process in the future.

- https://hub.docker.com/layers/accountlink/teleport/004/images/sha256-15cc5ebb7fb44234e8189cae5034908ebcfcb4872e9ba428001b7262e6e1a659?context=repo

### Listing certificates

The list of all certificates associated with the domain name `tee.teleport.best` can be found by running
```bash
python scripts/get-certs.py tee.teleport.best 2024-09-20
```

### Verifying the quote

To inspect a DCAP quote, we can use `gramine-sgx-quote-view`.

To validate the quote, we can use one of the tools like SGX-DCAP-verify.

TODO: add more detail here!

### Analyzing the software

The docker image includes the entire build environment.
You can modify the source code or build parameters, including building with the gramine simulator `gramine-direct`.
If you run the backend outside an enclave, it will run just fine except it will not output a quote.

