"""
SSL证书生成工具
"""

import os
import sys
import datetime
import ipaddress
import logging

logger = logging.getLogger("proxy")


def ensure_ssl_cert():
    """确保SSL证书存在，如不存在则生成"""

    # Python 3.6 compatibility
    timezone = datetime.timezone

    ssl_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ssl")
    cert_file = os.path.join(ssl_dir, "cert.pem")
    key_file = os.path.join(ssl_dir, "key.pem")

    # 如果证书已存在，直接返回
    if os.path.exists(cert_file) and os.path.exists(key_file):
        logger.info(f"SSL 证书已存在: {ssl_dir}")
        return cert_file, key_file

    # 创建SSL目录
    os.makedirs(ssl_dir, exist_ok=True)
    logger.info("正在生成自签名 SSL 证书 ...")

    try:
        from cryptography import x509
        from cryptography.x509.oid import NameOID
        from cryptography.hazmat.primitives import hashes, serialization
        from cryptography.hazmat.primitives.asymmetric import rsa

        # 生成私钥
        key = rsa.generate_private_key(public_exponent=65537, key_size=2048)

        # 生成证书主体
        subject = issuer = x509.Name([
            x509.NameAttribute(NameOID.COMMON_NAME, "api.anthropic.com"),
            x509.NameAttribute(NameOID.ORGANIZATION_NAME, "Claude Proxy"),
        ])

        # 生成证书
        now = datetime.datetime.now(timezone.utc)
        cert = (
            x509.CertificateBuilder()
            .subject_name(subject)
            .issuer_name(issuer)
            .public_key(key.public_key())
            .serial_number(x509.random_serial_number())
            .not_valid_before(now)
            .not_valid_after(now + datetime.timedelta(days=3650))
            .add_extension(
                x509.SubjectAlternativeName([
                    x509.DNSName("api.anthropic.com"),
                    x509.DNSName("*.anthropic.com"),
                    x509.DNSName("platform.claude.com"),
                    x509.DNSName("*.claude.com"),
                    x509.DNSName("localhost"),
                    x509.IPAddress(ipaddress.IPv4Address("127.0.0.1")),
                    x509.IPAddress(ipaddress.IPv4Address("0.0.0.0")),
                ]),
                critical=False,
            )
            .sign(key, hashes.SHA256())
        )

        # 保存私钥
        with open(key_file, "wb") as f:
            f.write(key.private_bytes(
                encoding=serialization.Encoding.PEM,
                format=serialization.PrivateFormat.TraditionalOpenSSL,
                encryption_algorithm=serialization.NoEncryption(),
            ))

        # 保存证书
        with open(cert_file, "wb") as f:
            f.write(cert.public_bytes(serialization.Encoding.PEM))

        logger.info(f"SSL 证书已生成: {cert_file}")
        return cert_file, key_file

    except ImportError:
        logger.error("缺少 cryptography 库! pip install cryptography")
        sys.exit(1)
