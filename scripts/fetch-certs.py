import sys
import requests
import json
from datetime import datetime
from cryptography import x509
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.hashes import SHA1
from cryptography.x509 import ocsp
from cryptography.x509.oid import ExtensionOID
import urllib.parse
import os

# Fetch certificate metadata from crt.sh
# If the expiry time matches our query date, fetch its contents
def fetch_certificates(domain, date):
    url = f"https://crt.sh/?match==&q={domain}&output=json"
    response = requests.get(url)
    certificates = []

    if response.status_code == 200:
        certs_data = json.loads(response.text)
        for cert_data in certs_data:
            not_before = datetime.strptime(cert_data['not_before'], "%Y-%m-%dT%H:%M:%S").date()
            not_after = datetime.strptime(cert_data['not_after'], "%Y-%m-%dT%H:%M:%S").date()
            
            if not_before <= date <= not_after:
                print(f'fetching {cert_data["id"]}')
                cert_pem = requests.get(f"https://crt.sh/?d={cert_data['id']}").text
                cert = x509.load_pem_x509_certificate(cert_pem.encode())
                certificates.append(cert)

    return certificates

# Check the OCSP to see if the certificate was revoked by the relevant date
def check_ocsp(cert, date):
    try:
        ocsp_extension = cert.extensions.get_extension_for_oid(ExtensionOID.AUTHORITY_INFORMATION_ACCESS)
        ocsp_url = next(desc.access_location.value for desc in ocsp_extension.value if desc.access_method.dotted_string == "1.3.6.1.5.5.7.48.1")
    except x509.ExtensionNotFound:
        print(f"No OCSP URL found for certificate: {cert.subject}")
        return None

    issuer_cert = fetch_issuer_certificate(cert)
    if not issuer_cert:
        print(f"Could not fetch issuer certificate for: {cert.subject}")
        return None

    builder = ocsp.OCSPRequestBuilder()
    builder = builder.add_certificate(cert, issuer_cert, SHA1())
    request = builder.build()

    headers = {
        "Content-Type": "application/ocsp-request",
        "Accept": "application/ocsp-response",
    }

    response = requests.post(ocsp_url, data=request.public_bytes(serialization.Encoding.DER), headers=headers)
    
    if response.status_code != 200:
        print(f"OCSP request failed for {cert.subject}")
        return None

    ocsp_response = ocsp.load_der_ocsp_response(response.content)    
    if ocsp_response.response_status != ocsp.OCSPResponseStatus.SUCCESSFUL:
        print(f"OCSP response not successful for {cert.subject}")
        return None

    cert_status = ocsp_response.certificate_status
    print(cert_status)
    
    if cert_status == ocsp.OCSPCertStatus.GOOD:
        return True
    elif cert_status == ocsp.OCSPCertStatus.REVOKED:
        revocation_time = ocsp_response.revocation_time.date()
        if date < revocation_time:
            return True
    
    return False

def fetch_issuer_certificate(cert):
    try:
        aia_extension = cert.extensions.get_extension_for_oid(ExtensionOID.AUTHORITY_INFORMATION_ACCESS)
        ca_issuers_url = next(desc.access_location.value for desc in aia_extension.value if desc.access_method.dotted_string == "1.3.6.1.5.5.7.48.2")
        
        response = requests.get(ca_issuers_url)
        return x509.load_der_x509_certificate(response.content)
    except Exception as e:
        print(f"Error fetching issuer certificate: {e}")
        return None

# Store all the relevant certificates in a directory for further inspection
def save_certificates(certificates, domain, date):
    os.makedirs(f"./{domain}_{date}/", exist_ok=True)
    for i, cert in enumerate(certificates):
        filename = f"{domain}_{date}/{domain}_{date}_{i}.pem"
        with open(filename, "wb") as f:
            f.write(cert.public_bytes(encoding=serialization.Encoding.PEM))
        print(f"Saved certificate: {filename}")

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python fetch-certs.py <domain> <date>")
        print("Example: python fetch-certs.py tee.teleport.best 2024-08-30")
        sys.exit(1)

    domain = sys.argv[1]
    date = datetime.strptime(sys.argv[2], "%Y-%m-%d").date()

    certificates = fetch_certificates(domain, date)
    active_certificates = [cert for cert in certificates if check_ocsp(cert, date)]
    save_certificates(active_certificates, domain, date)
    print(f"Found and saved {len(active_certificates)} active certificates for {domain} on {date}")
