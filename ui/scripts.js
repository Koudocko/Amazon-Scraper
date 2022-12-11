function fetchProduct(data = null){
	const { invoke } = window.__TAURI__.tauri 
	
	if (data == null){
		invoke('get_product', { lpn: document.getElementById("lpnInput").value })
			.then((result) =>{
				data = result;
		})
	}
	else{
		data = JSON.parse(data.replace(/[\r\n]/gm, ''));
	}
		
	if (data == null){
		data = ["None", "img/dotdotdot.jpg", "None", "None", "None"];
	}

	document.getElementById("productName").innerHTML = data[0];
	document.getElementById("productImage").setAttribute("src", data[1]);
	document.getElementById("productDescription").innerHTML = data[2];
	document.getElementById("productMSRP").innerHTML = data[3];
	document.getElementById("productASIN").innerHTML = data[4];
}

function writeProduct(){
	const { invoke } = window.__TAURI__.tauri

	var payload = JSON.parse('[]');
	payload.push(document.getElementById("productLOT").value);
	payload.push(document.getElementById("productName").innerHTML);
	payload.push(document.getElementById("productDescription").innerHTML);
	var temp = document.getElementById("productCondition");
	payload.push(temp.options[temp.selectedIndex].text);
	temp = document.getElementById("productVendor");
	payload.push(temp.options[temp.selectedIndex].text);
	payload.push("1");
	payload.push("3");
	temp = document.getElementById("productCategory");
	payload.push(temp.options[temp.selectedIndex].text);
	payload.push(document.getElementById("productMsrp").innerHTML);
	temp = document.getElementById("productImage");
	payload.push(temp.getAttribute("src"));

	invoke('write_product', { information: payload })
		.then((result) =>{
			if (result != null){
				var val = document.getElementById("productLOT"); 
				val.setAttribute("value", (parseInt(val.value) + 1).toString());
			}
	})
}

function findProduct(){
	const { invoke } = window.__TAURI__.tauri

	var div = document.getElementById("searchResults");
	div.innerHTML = "";

	invoke('find_product', { name: document.getElementById("nameInput").value })
		.then((result) =>{
			for (data of result){
				// div.innerHTML +=
				console.log('<div class="container" ' + "onClick='fetchProduct(`"
					+ JSON.stringify(data) + "`)'>" + 
					'<img style="height: 100px;" src="' 
					+ data[1] + `">` +
					`<div>
						<div>Name: <span>`
						+ data[0] + `<span></div>
						<div>ASIN: <span>`
						+ data[4] + `<span></div>
					</div>
				</div>`);
			}
	})
}
