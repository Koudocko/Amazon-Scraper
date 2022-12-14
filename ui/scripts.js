function fetchProduct(data = null, input = null){
	const { invoke } = window.__TAURI__.tauri 
	
	var inputVal;
	if (input == "UPC"){
		inputVal = document.getElementById("upcInput").value;
	}
	else if (input == "LPN"){
		inputVal = document.getElementById("lpnInput").value;
	}

	console.log(input);
	console.log(inputVal);

	if (data == null){
		invoke('get_product', { search: input, key: inputVal })
			.then((result) =>{
				data = result;

				if (data == null){
					data = ["", "img/dotdotdot.jpg", "", "", ""];
				}

				document.getElementById("productName").innerHTML = data[0];
				document.getElementById("productImage").setAttribute("src", data[1]);
				document.getElementById("productDescription").innerHTML = data[2];
				document.getElementById("productMSRP").innerHTML = data[3];
				document.getElementById("productASIN").innerHTML = data[4];
		})
	}
	else{
		data = JSON.parse(data.replace(/[\r\n]/gm, ''));

		if (data == null){
			data = ["", "img/dotdotdot.jpg", "", "", ""];
		}

		document.getElementById("productName").innerHTML = data[0];
		document.getElementById("productImage").setAttribute("src", data[1]);
		document.getElementById("productDescription").innerHTML = data[2];
		document.getElementById("productMSRP").innerHTML = data[3];
		document.getElementById("productASIN").innerHTML = data[4];
	}
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
	payload.push(document.getElementById("productMSRP").innerHTML);
	temp = document.getElementById("productImage");
	payload.push(temp.getAttribute("src"));

	invoke('write_product', { information: payload })
		.then((result) =>{
			if (result != null){
				console.log("good");
				var val = document.getElementById("productLOT"); 
				console.log(val.value);
				console.log((parseInt(val.value) + 1));
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
				div.innerHTML +=
				'<div class="search-result button" ' + "onClick='fetchProduct(`"
					+ JSON.stringify(data) + "`)'>" + 
					'<img class="search-result-img" src="' 
					+ data[1] + `">` +
					`<div>ASIN: <p>`
					+ data[4] + `</p></div>
					<div>Name: <p>`
					+ data[0] + `</p></div>
				</div>`;
			}
	})
}

function changePageBefore(path){
	const { invoke } = window.__TAURI__.tauri

	invoke('on_leave', 
		{ input: document.getElementById("inputStates").innerHTML, output: document.getElementById("outputState").innerHTML })
		.then(() =>{
			location.replace(path);
		})
}

function changePageAfter(){
	const { invoke } = window.__TAURI__.tauri

	invoke('on_load', {})
		.then((result) =>{
			if (result[0].length != 0){
				document.getElementById("inputStates").innerHTML = result[0];
			}
			if (result[1].length != 0){
				document.getElementById("outputState").innerHTML = result[1];

				if (document.getElementById("outputState").innerHTML  == "Loaded."){
					document.getElementById("outputState").style.color = 'var(--good)';
				}
			}
	})
}
