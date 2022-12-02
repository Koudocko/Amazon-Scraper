function fetchProduct(){
	const { invoke } = window.__TAURI__.tauri 

	invoke('get_product', { lpn: document.getElementById("lpnInput").value })
		.then((data) =>{
			if (data != null){
				document.getElementById("productName").innerHTML = data[0];
				document.getElementById("productImage").setAttribute("src", data[1]);
				document.getElementById("productDescription").innerHTML = data[2];
				document.getElementById("productMsrp").innerHTML = data[3];
			}
			else{
				document.getElementById("productName").innerHTML = "None";
				document.getElementById("productImage").setAttribute("src", "img/dotdotdot.jpg");
				document.getElementById("productDescription").innerHTML = "None";
				document.getElementById("productMsrp").innerHTML = "None";
			}
	})
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

	invoke('find_product', { name: document.getElementById("nameInput").value  })
		.then((result) =>{
			for (element of result){
				invoke('get_product', { lpn: element })
					.then((data) =>{
						if data != null{
							div.innerHTML += 
							`<div class="container foreground">
									<img height="100px" class="container" src="`
								+ data[1] + '">' +
								"<div>";
						}
				})
			}
	})
}
