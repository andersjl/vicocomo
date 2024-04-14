const { invoke } = window.__TAURI__.tauri
//invoke("log", { msg: "here" });

const invoke_request = (evnt, method, url, body) => {
  if (evnt) {
    evnt.preventDefault();
  }
  invoke("request", { method: method, url: url, body: body })
    .then((response) => {
      if (response[0]) {
          document.querySelector("html").innerHTML = response[1];
          capture_links();
          capture_forms();
          capture_submit_onchange();
      }
    })
}

const capture_links = () => {
  let links = document.querySelectorAll("a");
  links.forEach((link) => {
    link.onclick = (e) => {
      invoke_request(e, "get", e.target.href, "");
    }
  });
}

const send_form = (e, form) => {
  let bdy = "";
  let formData = new FormData(form);
  for (let [name, value] of formData) {
    console.log(name, value);
    if (bdy.length > 0) {
      bdy += "&";
    }
    bdy += name + "=" + value;
  }
  invoke_request(
    e,
    form.method,
    form.action,
    bdy,
  );
}

const capture_forms = () => {
  let forms = document.querySelectorAll("form");
  forms.forEach((form) => {
    if (form.getAttribute("enctype") == "multipart/form-data"
      && form.method.toUpperCase() == "POST"
    ) {
      // Hack to prevent the UI to load the file:
      // - Hide the submit button
      // - Make the file control send a __VICOCOMO__upload... request
      // - The file is loaded by vicocomo_tauri::fix_body()
      for (let ix = 0; ix < form.elements.length; ix++) {
        let elem = form.elements[ix];
        switch (elem.getAttribute("type")) {
          case "submit":
          case "button":
            elem.setAttribute("type", "hidden");
            break;
          case "file":
            elem.setAttribute("action", form.action);
            elem.onclick = (e) => {
              let bdy = "__VICOCOMO__upload_name=" + e.target.name;
              if (e.target.getAttribute("multiple") !== null) {
                bdy = bdy + "&" + "__VICOCOMO__upload_multiple";
              }
              invoke_request(e, "POST", e.target.getAttribute("action"), bdy);
            }
            break;
          default:
        }
      }
    } else {
      form.onsubmit = (e) => {
        send_form(e, e.target);
      }
    }
  });
}

const capture_submit_onchange = () => {
  let submitters = document.querySelectorAll(".vicocomo--submit-on-change");
  submitters.forEach((submitter) => {
    submitter.onchange = (e) => {
      send_form(null, e.target.form);
    }
  })
}

