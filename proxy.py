import flask
import requests

app = flask.Flask(__name__)

@app.route('/', defaults={'path': ''}, methods=['GET', 'POST', 'PUT', 'DELETE'])
@app.route('/<path:path>', methods=['GET', 'POST', 'PUT', 'DELETE'])
def proxy(path):
    print(f"=== REQ: {flask.request.method} {path} ===")
    if flask.request.method == 'POST':
        print(flask.request.get_data().decode('utf-8'))
        
    resp = requests.request(
        method=flask.request.method,
        url=f"http://127.0.0.1:8080/{path}",
        headers={key: value for (key, value) in flask.request.headers if key != 'Host'},
        data=flask.request.get_data(),
        cookies=flask.request.cookies,
        allow_redirects=False)
        
    print(f"=== RES: {resp.status_code} ===")
    print(resp.text)
    
    return flask.Response(resp.content, resp.status_code, resp.headers.items())

if __name__ == '__main__':
    app.run(port=8081, debug=False)
