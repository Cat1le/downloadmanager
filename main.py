import requests

r = requests.get('https://downloads.mitmproxy.org/10.3.1/mitmproxy-10.3.1.tar.gz')
print(r.request.headers)