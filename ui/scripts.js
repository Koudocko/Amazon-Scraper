function fetchProduct(data = null){
	const { invoke } = window.__TAURI__.tauri 
	
	if (data == null){
		invoke('get_product', { lpn: document.getElementById("lpnInput").value })
			.then((result) =>{
				data = result;
		})
	}
	else{
		console.log(data);
		data = JSON.parse(data.replace(/[\r\n]/gm, ''));
	}
		
	if (data == null){
		data[0] = "None";
		data[1] = "img/dotdotdot.jpg";
		data[2] = "None";
		data[3] = "None";
	}

	document.getElementById("productName").innerHTML = data[0];
	document.getElementById("productImage").setAttribute("src", data[1]);
	document.getElementById("productDescription").innerHTML = data[2];
	document.getElementById("productMsrp").innerHTML = data[3];
}

function writeProduct(){
	const { invoke } = window.__TAURI__.tauri

	var payload = JSON.parse('[]');
	payload.push(document.getElementById("productLot").value);
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
				var val = document.getElementById("productLot").value; 
				document.getElementById("productLot").setAttribute("value", (parseInt(val) + 1).toString());
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
				div.innerHTML +=
				'<div class="container" ' + "onClick='fetchProduct(`"
					+ JSON.stringify(data) + "`)'" + 'style="outline: red solid 5px;">' +
					'<img style="height: 100px;" src="' 
					+ data[1] + `">` +
					`<div>
						<div>Name: <span>`
						+ data[0] + `<span></div>
						<div>ASIN: <span>`
						+ data[4] + `<span></div>
					</div>
				</div>`;
			}
	})
}
