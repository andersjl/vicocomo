const { invoke } = window.__TAURI__.tauri
//invoke('log', { msg: 'here' });

const invoke_request = (evnt, method, url, body) => {
  if (evnt) {
    evnt.preventDefault();
  }
  invoke('request', { method: method, url: url, body: body })
    .then((response) => {
      if (response[0]) {
          document.querySelector('html').innerHTML = response[1];
          capture_links();
          capture_forms();
      }
    })
}

const capture_links = () => {
  let links = document.querySelectorAll('a');
  links.forEach((link) => {
    link.onclick = function(e) {
      invoke_request(e, 'get', e.target.href, '');
    }
  });
}

const capture_forms = () => {
  let forms = document.querySelectorAll('form');
  forms.forEach((form) => {
    form.onsubmit = function(e) {
      let body = '';
      for (let ix = 0; ix < e.target.elements.length; ix++) {
        let elem = e.target.elements[ix];
        if (body.length > 0) {
          body += '&';
        }
        body += elem.name + '=' + elem.value;
      }
      invoke_request(
        e,
        e.target.method,
        e.target.action,
        body,
      );
    }
  });
}
